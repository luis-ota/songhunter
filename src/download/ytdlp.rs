use crate::error::AppError;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::process::Command;
use tracing::{debug, info, warn};

pub struct Downloader {
    ytdlp_path: String,
    temp_dir: PathBuf,
    extra_args: Vec<String>,
    cookies_file: Option<String>,
    client: reqwest::Client,
}

impl Downloader {
    pub fn new(
        ytdlp_path: impl Into<String>,
        temp_dir: impl AsRef<Path>,
        extra_args: Vec<String>,
        cookies_file: Option<String>,
    ) -> Self {
        Self {
            ytdlp_path: ytdlp_path.into(),
            temp_dir: temp_dir.as_ref().to_path_buf(),
            extra_args,
            cookies_file,
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(15))
                .redirect(reqwest::redirect::Policy::none())
                .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36")
                .build()
                .expect("failed to build reqwest client"),
        }
    }

    /// Follow one redirect to resolve short URLs (e.g. vt.tiktok.com).
    async fn resolve_one_redirect(&self, url: &str) -> String {
        if let Ok(resp) = self.client.get(url).send().await {
            if let Some(location) = resp.headers().get(reqwest::header::LOCATION) {
                if let Ok(loc) = location.to_str() {
                    let resolved = if loc.starts_with("http://") || loc.starts_with("https://") {
                        loc.to_string()
                    } else {
                        format!("{}{}", url.trim_end_matches('/'), loc)
                    };
                    info!(original = url, resolved, "followed redirect");
                    return resolved;
                }
            }
        }
        url.to_string()
    }

    /// Replace /photo/ with /video/ in TikTok URLs (same ID, works with yt-dlp).
    fn fix_tiktok_photo(url: &str) -> String {
        if url.contains("/photo/") && url.contains("tiktok.com") {
            let fixed = url.replace("/photo/", "/video/");
            info!(original = url, fixed, "fixed TikTok photo URL -> video");
            fixed
        } else {
            url.to_string()
        }
    }

    /// Drop tracking query/fragment from Instagram links and normalize /reels/.
    fn normalize_instagram(url: &str) -> String {
        if !url.contains("instagram.com") {
            return url.to_string();
        }
        let sem_query = url.split(['?', '#']).next().unwrap_or(url);
        let normalizada = sem_query.replace("/reels/", "/reel/");
        if normalizada != url {
            info!(original = url, normalizada, "normalized Instagram URL");
        }
        normalizada
    }

    /// Fallback TikTok via TikWM quando o challenge do yt-dlp falha.
    async fn tikwm_fallback(&self, url: &str, output_dir: &Path) -> Result<PathBuf, AppError> {
        info!(url, "trying TikWM fallback");
        let api = reqwest::Url::parse_with_params("https://www.tikwm.com/api/", &[("url", url)])
            .map_err(|e| AppError::Download(format!("invalid TikWM query: {e}")))?;

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36")
            .build()
            .map_err(|e| AppError::Download(format!("failed to build http client: {e}")))?;

        let resp = client
            .get(api)
            .send()
            .await
            .map_err(|e| AppError::Download(format!("TikWM request failed: {e}")))?;
        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| AppError::Download(format!("TikWM returned invalid JSON: {e}")))?;

        if data.get("code").and_then(|c| c.as_i64()) != Some(0) {
            let msg = data
                .get("msg")
                .and_then(|m| m.as_str())
                .unwrap_or("unknown error");
            return Err(AppError::Download(format!("TikWM error: {msg}")));
        }

        let play = data
            .pointer("/data/play")
            .and_then(|p| p.as_str())
            .ok_or_else(|| AppError::Download("TikWM response missing data.play".to_string()))?;

        let media = client
            .get(play)
            .header(reqwest::header::REFERER, "https://www.tiktok.com/")
            .send()
            .await
            .map_err(|e| AppError::Download(format!("TikWM media download failed: {e}")))?;
        if !media.status().is_success() {
            return Err(AppError::Download(format!(
                "TikWM media returned status {}",
                media.status()
            )));
        }

        let bytes = media
            .bytes()
            .await
            .map_err(|e| AppError::Download(format!("TikWM media read failed: {e}")))?;
        let path = output_dir.join("tiktok-fallback.mp4");
        tokio::fs::write(&path, bytes)
            .await
            .map_err(|e| AppError::Download(format!("failed to write TikWM media: {e}")))?;
        info!(path = %path.display(), "TikWM fallback media saved");
        Ok(path)
    }

    async fn run_ytdlp(
        &self,
        url: &str,
        output_dir: &Path,
        output_template: &str,
    ) -> Result<PathBuf, AppError> {
        let mut cmd = Command::new(&self.ytdlp_path);
        cmd.args([
            "--no-playlist",
            "--extract-audio",
            "--audio-format",
            "wav",
            "--audio-quality",
            "0",
            "--output",
            output_template,
            "--newline",
            "--no-warnings",
        ]);

        if let Some(cookies) = &self.cookies_file {
            cmd.arg("--cookies");
            cmd.arg(cookies);
        }

        for arg in &self.extra_args {
            cmd.arg(arg);
        }

        cmd.arg(url);

        debug!(?cmd, "running yt-dlp");
        let output = cmd
            .output()
            .await
            .map_err(|e| AppError::Download(format!("yt-dlp failed to execute: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::Download(format!(
                "yt-dlp exited with status {}: {stderr}",
                output.status
            )));
        }

        let mut entries = tokio::fs::read_dir(output_dir)
            .await
            .map_err(|e| AppError::Download(format!("failed to read output dir: {e}")))?;

        let mut wav_path: Option<PathBuf> = None;
        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| AppError::Download(format!("failed to list dir: {e}")))?
        {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("wav") {
                wav_path = Some(path);
                break;
            }
        }

        wav_path.ok_or_else(|| AppError::Download("yt-dlp did not produce a wav file".to_string()))
    }

    pub async fn download(&self, url: &str, task_id: &str) -> Result<PathBuf, AppError> {
        info!(task_id, url, "starting download");
        let output_dir = self.temp_dir.join(task_id);
        tokio::fs::create_dir_all(&output_dir)
            .await
            .map_err(|e| AppError::Download(format!("failed to create temp dir: {e}")))?;

        let output_template = output_dir.join("%(id)s.%(ext)s");
        let output_template_str = output_template.to_string_lossy();

        let effective_url = if url.contains("tiktok.com") {
            let resolved = self.resolve_one_redirect(url).await;
            Self::fix_tiktok_photo(&resolved)
        } else if url.contains("instagram.com") {
            Self::normalize_instagram(url)
        } else {
            url.to_string()
        };

        match self
            .run_ytdlp(&effective_url, &output_dir, &output_template_str)
            .await
        {
            Ok(path) => Ok(path),
            Err(err) => {
                if effective_url.contains("tiktok.com") {
                    warn!(task_id, url = effective_url, error = %err, "yt-dlp failed, falling back to TikWM");
                    self.tikwm_fallback(&effective_url, &output_dir).await
                } else {
                    Err(err)
                }
            }
        }
    }

    pub async fn cleanup(&self, task_id: &str) -> anyhow::Result<()> {
        let dir = self.temp_dir.join(task_id);
        if dir.exists() {
            tokio::fs::remove_dir_all(&dir).await?;
        }
        Ok(())
    }
}

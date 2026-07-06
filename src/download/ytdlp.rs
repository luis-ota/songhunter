use crate::error::AppError;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::process::Command;
use tracing::{debug, info};

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
        } else {
            url.to_string()
        };

        self.run_ytdlp(&effective_url, &output_dir, &output_template_str)
            .await
    }

    pub async fn cleanup(&self, task_id: &str) -> anyhow::Result<()> {
        let dir = self.temp_dir.join(task_id);
        if dir.exists() {
            tokio::fs::remove_dir_all(&dir).await?;
        }
        Ok(())
    }
}

use crate::error::AppError;
use std::path::{Path, PathBuf};
use tokio::process::Command;
use tracing::{debug, info};

pub struct Downloader {
    ytdlp_path: String,
    temp_dir: PathBuf,
    extra_args: Vec<String>,
}

impl Downloader {
    pub fn new(
        ytdlp_path: impl Into<String>,
        temp_dir: impl AsRef<Path>,
        extra_args: Vec<String>,
    ) -> Self {
        Self {
            ytdlp_path: ytdlp_path.into(),
            temp_dir: temp_dir.as_ref().to_path_buf(),
            extra_args,
        }
    }

    pub async fn download(&self, url: &str, task_id: &str) -> Result<PathBuf, AppError> {
        info!(task_id, url, "starting download");
        let output_dir = self.temp_dir.join(task_id);
        tokio::fs::create_dir_all(&output_dir)
            .await
            .map_err(|e| AppError::Download(format!("failed to create temp dir: {e}")))?;

        let output_template = output_dir.join("%(id)s.%(ext)s");
        let output_template_str = output_template.to_string_lossy();

        let mut cmd = Command::new(&self.ytdlp_path);
        cmd.args([
            "--no-playlist",
            "--extract-audio",
            "--audio-format",
            "wav",
            "--audio-quality",
            "0",
            "--output",
            &output_template_str,
            "--newline",
            "--no-warnings",
        ]);

        // Args extras configuráveis via .env
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

        let mut entries = tokio::fs::read_dir(&output_dir)
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

    pub async fn cleanup(&self, task_id: &str) -> anyhow::Result<()> {
        let dir = self.temp_dir.join(task_id);
        if dir.exists() {
            tokio::fs::remove_dir_all(&dir).await?;
        }
        Ok(())
    }
}

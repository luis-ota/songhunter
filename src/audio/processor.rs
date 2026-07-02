use crate::error::AppError;
use std::path::{Path, PathBuf};
use tokio::process::Command;
use tracing::{debug, info};

pub struct AudioProcessor {
    ffmpeg_path: String,
    fpcalc_path: String,
    temp_dir: PathBuf,
}

impl AudioProcessor {
    pub fn new(
        ffmpeg_path: impl Into<String>,
        fpcalc_path: impl Into<String>,
        temp_dir: impl AsRef<Path>,
    ) -> Self {
        Self {
            ffmpeg_path: ffmpeg_path.into(),
            fpcalc_path: fpcalc_path.into(),
            temp_dir: temp_dir.as_ref().to_path_buf(),
        }
    }

    pub async fn normalize(
        &self,
        input: impl AsRef<Path>,
        task_id: &str,
    ) -> Result<PathBuf, AppError> {
        info!(task_id, "normalizing audio");
        let output_dir = self.temp_dir.join(task_id);
        tokio::fs::create_dir_all(&output_dir)
            .await
            .map_err(|e| AppError::Conversion(format!("failed to create temp dir: {e}")))?;

        let output = output_dir.join("normalized.wav");

        let mut cmd = Command::new(&self.ffmpeg_path);
        cmd.args([
            "-y",
            "-i",
            input.as_ref().to_str().unwrap_or(""),
            "-t",
            "90",
            "-ar",
            "44100",
            "-ac",
            "1",
            "-sample_fmt",
            "s16",
            output.to_string_lossy().as_ref(),
        ]);

        debug!(?cmd, "running ffmpeg");
        let result = cmd
            .output()
            .await
            .map_err(|e| AppError::Conversion(format!("ffmpeg failed to execute: {e}")))?;

        if !result.status.success() {
            let stderr = String::from_utf8_lossy(&result.stderr);
            return Err(AppError::Conversion(format!(
                "ffmpeg exited with status {}: {stderr}",
                result.status
            )));
        }

        Ok(output)
    }

    pub async fn fingerprint(
        &self,
        wav_path: impl AsRef<Path>,
        task_id: &str,
    ) -> Result<(String, i32), AppError> {
        info!(task_id, "computing chromaprint");
        let mut cmd = Command::new(&self.fpcalc_path);
        cmd.args([
            "-json",
            "-length",
            "90",
            wav_path.as_ref().to_string_lossy().as_ref(),
        ]);

        let result = cmd
            .output()
            .await
            .map_err(|e| AppError::Identification(format!("fpcalc failed to execute: {e}")))?;

        let parsed: Result<serde_json::Value, _> = serde_json::from_slice(&result.stdout);

        let parsed = match parsed {
            Ok(v) => v,
            Err(e) => {
                let stderr = String::from_utf8_lossy(&result.stderr);
                return Err(AppError::Identification(format!(
                    "fpcalc failed to produce valid JSON (exit: {}, stderr: {stderr}): {e}",
                    result.status
                )));
            }
        };

        let fp = parsed["fingerprint"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| {
                AppError::Identification("fpcalc did not return a fingerprint".to_string())
            })?;

        let duration = parsed["duration"]
            .as_f64()
            .map(|d| d as i32)
            .unwrap_or(90);

        Ok((fp, duration))
    }

    pub async fn cleanup(&self, task_id: &str) -> anyhow::Result<()> {
        let dir = self.temp_dir.join(task_id);
        if dir.exists() {
            tokio::fs::remove_dir_all(&dir).await?;
        }
        Ok(())
    }
}

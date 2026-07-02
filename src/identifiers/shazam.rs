use crate::config::ShazamConfig;
use crate::identifiers::backend::Identifier;
use crate::models::SongMatch;
use async_trait::async_trait;
use std::path::Path;
use tokio::process::Command;
use tracing::{info, warn};

pub struct ShazamIdentifier {
    python_path: String,
    script_path: String,
}

impl ShazamIdentifier {
    pub fn new(_config: ShazamConfig) -> Self {
        Self {
            python_path: "python3".into(),
            script_path: "/app/scripts/shazam_recognize.py".into(),
        }
    }
}

#[async_trait]
impl Identifier for ShazamIdentifier {
    fn name(&self) -> &'static str {
        "shazam"
    }

    fn enabled(&self) -> bool {
        true
    }

    async fn identify(&self, wav_path: &Path) -> anyhow::Result<Vec<SongMatch>> {
        info!("identifying with Shazam");

        let mut cmd = Command::new(&self.python_path);
        cmd.args([
            self.script_path.as_str(),
            wav_path.to_string_lossy().as_ref(),
        ]);

        let output = match cmd.output().await {
            Ok(o) => o,
            Err(e) => {
                warn!("shazam subprocess failed to execute: {}", e);
                return Ok(Vec::new());
            }
        };

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("shazam subprocess exited with {}: {stderr}", output.status);
            return Ok(Vec::new());
        }

        let data: serde_json::Value = match serde_json::from_slice(&output.stdout) {
            Ok(v) => v,
            Err(e) => {
                warn!("shazam subprocess returned invalid JSON: {}", e);
                return Ok(Vec::new());
            }
        };

        if data.get("error").is_some() {
            warn!("shazam subprocess error: {:?}", data["error"]);
            return Ok(Vec::new());
        }

        let title = match data["title"].as_str() {
            Some(t) => t.to_string(),
            None => return Ok(Vec::new()),
        };

        let artist = data["artist"].as_str().unwrap_or("Unknown").to_string();
        let album = data["album"].as_str().map(|s| s.to_string());
        let artwork_url = data["artwork_url"].as_str().map(|s| s.to_string());
        let preview_url = data["preview_url"].as_str().map(|s| s.to_string());
        let isrc = data["isrc"].as_str().map(|s| s.to_string());
        let confidence = data["score"].as_f64().unwrap_or(0.8) as f32;

        Ok(vec![SongMatch {
            rank: None,
            title,
            artist,
            album,
            confidence,
            provider: "shazam".to_string(),
            artwork_url,
            preview_url,
            isrc,
            duration: None,
        }])
    }
}

use crate::config::ShazamConfig;
use crate::identifiers::backend::Identifier;
use crate::models::SongMatch;
use async_trait::async_trait;
use std::path::Path;
use tracing::{info, warn};

pub struct ShazamIdentifier {
    config: ShazamConfig,
    client: reqwest::Client,
}

impl ShazamIdentifier {
    pub fn new(config: ShazamConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .expect("reqwest client"),
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
        let audio_bytes = tokio::fs::read(wav_path).await?;

        let uuid = uuid::Uuid::new_v4();
        let url = format!("{}/{}", self.config.base_url.trim_end_matches('/'), uuid);

        let part = reqwest::multipart::Part::bytes(audio_bytes)
            .file_name("audio.wav")
            .mime_str("audio/wav")?;

        let form = reqwest::multipart::Form::new().part("audio", part);

        let resp = match self
            .client
            .post(&url)
            .header("User-Agent", "shazamio/0.0.1")
            .multipart(form)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                warn!("Shazam request failed: {}", e);
                return Ok(Vec::new());
            }
        };

        if !resp.status().is_success() {
            warn!("Shazam returned status {}", resp.status());
            return Ok(Vec::new());
        }

        let data: serde_json::Value = resp.json().await?;
        let mut matches = Vec::new();

        if let Some(track) = data.get("track").and_then(|t| t.as_object()) {
            let title = track
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown")
                .to_string();
            let artist = track
                .get("subtitle")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown")
                .to_string();
            let artwork_url = track
                .get("images")
                .and_then(|i| i.get("coverarthq"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let preview_url = track
                .get("hub")
                .and_then(|h| h.get("actions"))
                .and_then(|a| a.as_array())
                .and_then(|arr| {
                    arr.iter()
                        .find(|x| x.get("type").and_then(|t| t.as_str()) == Some("uri"))
                })
                .and_then(|x| x.get("uri"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let score = data
                .get("matches")
                .and_then(|m| m.as_array())
                .and_then(|arr| arr.first())
                .and_then(|first| first.get("score"))
                .and_then(|v| v.as_f64())
                .unwrap_or(0.8) as f32;

            matches.push(SongMatch {
                rank: None,
                title,
                artist,
                album: None,
                confidence: score.min(1.0),
                provider: "shazam".to_string(),
                artwork_url,
                preview_url,
                isrc: None,
                duration: None,
            });
        }

        Ok(matches)
    }
}

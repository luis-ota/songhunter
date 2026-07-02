use crate::config::AcrCloudConfig;
use crate::identifiers::backend::Identifier;
use crate::models::SongMatch;
use async_trait::async_trait;
use base64::Engine;
use hmac::{Hmac, Mac};
use sha1::Sha1;
use std::path::Path;
use tracing::{info, warn};

type HmacSha1 = Hmac<Sha1>;

pub struct AcrCloudIdentifier {
    config: AcrCloudConfig,
    client: reqwest::Client,
}

impl AcrCloudIdentifier {
    pub fn new(config: AcrCloudConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(20))
                .build()
                .expect("reqwest client"),
        }
    }

    fn build_signature(&self, timestamp: &str) -> anyhow::Result<String> {
        let string_to_sign = format!(
            "POST\n/v1/identify\n{}\n{}",
            self.config.access_key, timestamp
        );
        let mut mac = HmacSha1::new_from_slice(self.config.access_secret.as_bytes())?;
        mac.update(string_to_sign.as_bytes());
        let result = mac.finalize();
        let bytes = result.into_bytes();
        Ok(base64::engine::general_purpose::STANDARD.encode(bytes))
    }
}

#[async_trait]
impl Identifier for AcrCloudIdentifier {
    fn name(&self) -> &'static str {
        "acrcloud"
    }

    fn enabled(&self) -> bool {
        !self.config.access_key.is_empty()
            && !self.config.access_secret.is_empty()
            && !self.config.host.is_empty()
    }

    async fn identify(&self, wav_path: &Path) -> anyhow::Result<Vec<SongMatch>> {
        info!("identifying with ACRCloud");
        let audio_bytes = tokio::fs::read(wav_path).await?;
        let sample_bytes = audio_bytes.len().to_string();

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs()
            .to_string();

        let signature = self.build_signature(&timestamp)?;
        let url = format!("https://{}/v1/identify", self.config.host);

        let part = reqwest::multipart::Part::bytes(audio_bytes)
            .file_name("audio.wav")
            .mime_str("audio/wav")?;

        let form = reqwest::multipart::Form::new()
            .text("access_key", self.config.access_key.clone())
            .text("sample_bytes", sample_bytes)
            .text("timestamp", timestamp)
            .text("signature", signature)
            .text("data_type", "audio")
            .text("signature_version", "1")
            .part("sample", part);

        let resp = match self.client.post(&url).multipart(form).send().await {
            Ok(r) => r,
            Err(e) => {
                warn!("ACRCloud request failed: {}", e);
                return Ok(Vec::new());
            }
        };

        if !resp.status().is_success() {
            warn!("ACRCloud returned status {}", resp.status());
            return Ok(Vec::new());
        }

        let data: serde_json::Value = resp.json().await?;
        let mut matches = Vec::new();

        if let Some(metas) = data.get("metadata").and_then(|m| m.as_object()) {
            for (kind, value) in metas {
                let items = match value.as_array() {
                    Some(arr) => arr,
                    None => continue,
                };
                for item in items.iter().take(2) {
                    let title = item
                        .get("title")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown")
                        .to_string();
                    let artist = item
                        .get("artists")
                        .and_then(|a| a.as_array())
                        .and_then(|arr| arr.first())
                        .and_then(|first| first.get("name"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown")
                        .to_string();
                    let album = item
                        .get("album")
                        .and_then(|v| v.get("name"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    let score = item.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let duration = item
                        .get("duration_ms")
                        .and_then(|v| v.as_f64())
                        .map(|d| (d / 1000.0) as f32);

                    matches.push(SongMatch {
                        rank: None,
                        title,
                        artist,
                        album,
                        confidence: score.min(1.0),
                        provider: format!("acrcloud/{kind}"),
                        artwork_url: None,
                        preview_url: None,
                        isrc: None,
                        duration,
                    });
                }
            }
        }

        matches.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        Ok(matches)
    }
}

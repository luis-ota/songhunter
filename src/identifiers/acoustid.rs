use crate::audio::AudioProcessor;
use crate::config::AcoustIdConfig;
use crate::identifiers::backend::Identifier;
use crate::models::SongMatch;
use async_trait::async_trait;
use std::path::Path;
use tracing::{info, warn};

pub struct AcoustIdIdentifier {
    config: AcoustIdConfig,
    client: reqwest::Client,
}

impl AcoustIdIdentifier {
    pub fn new(config: AcoustIdConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(20))
                .build()
                .expect("reqwest client"),
        }
    }

    fn extract_recording_matches(&self, result: &serde_json::Value, score: f32) -> Vec<SongMatch> {
        let mut matches = Vec::new();
        let recordings = match result.get("recordings").and_then(|r| r.as_array()) {
            Some(r) => r,
            None => return matches,
        };

        for recording in recordings.iter().take(2) {
            let title = recording
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown")
                .to_string();
            let artist = recording
                .get("artists")
                .and_then(|a| a.as_array())
                .and_then(|arr| arr.first())
                .and_then(|first| first.get("name"))
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown")
                .to_string();
            let album = recording
                .get("releasegroups")
                .and_then(|rg| rg.as_array())
                .and_then(|arr| arr.first())
                .and_then(|first| first.get("title"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let duration = recording
                .get("duration")
                .and_then(|v| v.as_f64())
                .map(|d| d as f32);

            matches.push(SongMatch {
                rank: None,
                title,
                artist,
                album,
                confidence: score.min(1.0),
                provider: "acoustid".to_string(),
                artwork_url: None,
                preview_url: None,
                isrc: None,
                duration,
            });
        }

        matches
    }
}

#[async_trait]
impl Identifier for AcoustIdIdentifier {
    fn name(&self) -> &'static str {
        "acoustid"
    }

    fn enabled(&self) -> bool {
        !self.config.api_key.is_empty()
    }

    async fn identify(&self, wav_path: &Path) -> anyhow::Result<Vec<SongMatch>> {
        info!("identifying with AcoustID");

        let proc = AudioProcessor::new("ffmpeg", &self.config.fpcalc_path, std::env::temp_dir());
        let fingerprint = proc.fingerprint(wav_path, "acoustid").await?;

        let params = [
            ("client", self.config.api_key.as_str()),
            ("fingerprint", fingerprint.as_str()),
            (
                "meta",
                "recordings sources releasegroups releases tracks compress",
            ),
            ("format", "json"),
        ];

        let resp = match self
            .client
            .get(&self.config.url)
            .query(&params)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                warn!("AcoustID request failed: {}", e);
                return Ok(Vec::new());
            }
        };

        if !resp.status().is_success() {
            warn!("AcoustID returned status {}", resp.status());
            return Ok(Vec::new());
        }

        let data: serde_json::Value = resp.json().await?;
        let mut matches = Vec::new();

        let results = match data.get("results").and_then(|r| r.as_array()) {
            Some(r) => r,
            None => return Ok(Vec::new()),
        };

        for result in results.iter().take(4) {
            let score = result.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            matches.extend(self.extract_recording_matches(result, score));
        }

        matches.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        Ok(matches)
    }
}

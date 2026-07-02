use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentifyRequest {
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentifyResponse {
    pub task_id: String,
    pub status: TaskStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    #[default]
    Queued,
    Downloading,
    Converting,
    Identifying,
    Done,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaskResult {
    pub task_id: String,
    pub status: TaskStatus,
    pub progress: f32,
    pub message: String,
    pub songs: Vec<SongMatch>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SongMatch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rank: Option<usize>,
    pub title: String,
    pub artist: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub album: Option<String>,
    pub confidence: f32,
    pub provider: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artwork_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub isrc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<f32>,
}

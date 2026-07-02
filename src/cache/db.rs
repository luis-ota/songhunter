use crate::models::{SongMatch, TaskResult};
use anyhow::Result;
use chrono::{Duration, Utc};
use sha2::{Digest, Sha256};
use sqlx::{Pool, Sqlite, sqlite::SqlitePoolOptions};
use std::path::Path;

pub struct Cache {
    pool: Pool<Sqlite>,
    ttl_hours: i64,
}

impl Cache {
    pub async fn new(db_path: impl AsRef<Path>, ttl_hours: i64) -> Result<Self> {
        let pool = SqlitePoolOptions::new()
            .max_connections(16)
            .connect(&format!(
                "sqlite:{}?mode=rwc",
                db_path.as_ref().to_string_lossy()
            ))
            .await?;

        sqlx::migrate!("./migrations").run(&pool).await?;

        Ok(Self { pool, ttl_hours })
    }

    pub fn audio_hash(audio_path: impl AsRef<Path>) -> Result<String> {
        let bytes = std::fs::read(audio_path)?;
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        Ok(hex::encode(hasher.finalize()))
    }

    pub async fn get_audio_result(&self, audio_hash: &str) -> Result<Option<Vec<SongMatch>>> {
        let cutoff = Utc::now() - Duration::hours(self.ttl_hours);

        let row: Option<(String,)> =
            sqlx::query_as("SELECT songs FROM audio_cache WHERE audio_hash = ? AND created_at > ?")
                .bind(audio_hash)
                .bind(cutoff)
                .fetch_optional(&self.pool)
                .await?;

        match row {
            Some((json,)) => Ok(Some(serde_json::from_str(&json)?)),
            None => Ok(None),
        }
    }

    pub async fn set_audio_result(&self, audio_hash: &str, songs: &[SongMatch]) -> Result<()> {
        let json = serde_json::to_string(songs)?;
        sqlx::query(
            "INSERT OR REPLACE INTO audio_cache (audio_hash, songs, created_at) VALUES (?, ?, ?)",
        )
        .bind(audio_hash)
        .bind(json)
        .bind(Utc::now())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_task(&self, task_id: &str) -> Result<Option<TaskResult>> {
        let row: Option<(String,)> =
            sqlx::query_as("SELECT payload FROM task_cache WHERE task_id = ?")
                .bind(task_id)
                .fetch_optional(&self.pool)
                .await?;

        match row {
            Some((json,)) => Ok(Some(serde_json::from_str(&json)?)),
            None => Ok(None),
        }
    }

    pub async fn set_task(&self, result: &TaskResult) -> Result<()> {
        let json = serde_json::to_string(result)?;
        sqlx::query(
            "INSERT OR REPLACE INTO task_cache (task_id, payload, updated_at) VALUES (?, ?, ?)",
        )
        .bind(&result.task_id)
        .bind(json)
        .bind(Utc::now())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn cleanup(&self) -> Result<u64> {
        let cutoff = Utc::now() - Duration::hours(self.ttl_hours);
        let rows = sqlx::query("DELETE FROM audio_cache WHERE created_at < ?")
            .bind(cutoff)
            .execute(&self.pool)
            .await?;
        Ok(rows.rows_affected())
    }
}

use crate::models::SongMatch;
use async_trait::async_trait;
use std::path::Path;

#[async_trait]
pub trait Identifier: Send + Sync {
    fn name(&self) -> &'static str;
    fn enabled(&self) -> bool;
    async fn identify(&self, wav_path: &Path) -> anyhow::Result<Vec<SongMatch>>;
}

use crate::ads::AdEngine;
use crate::cache::Cache;
use crate::config::Config;
use crate::identifiers::IdentifierRegistry;
use std::path::PathBuf;
use std::sync::Arc;

pub struct AppState {
    #[allow(dead_code)]
    pub config: Arc<Config>,
    pub cache: Arc<Cache>,
    pub registry: Arc<IdentifierRegistry>,
    pub task_tx: tokio::sync::mpsc::Sender<crate::worker::TaskCommand>,
    pub cookies_file: PathBuf,
    pub ad_engine: AdEngine,
}

impl AppState {
    pub fn new(
        config: Config,
        cache: Arc<Cache>,
        registry: Arc<IdentifierRegistry>,
        task_tx: tokio::sync::mpsc::Sender<crate::worker::TaskCommand>,
    ) -> Self {
        let cookies_file = config
            .ytdlp_cookies_file
            .as_ref()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("./data/cookies.txt"));
        let ad_engine = AdEngine::new(config.ads.clone());
        Self {
            config: Arc::new(config),
            cache,
            registry,
            task_tx,
            cookies_file,
            ad_engine,
        }
    }
}

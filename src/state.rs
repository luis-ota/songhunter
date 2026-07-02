use crate::cache::Cache;
use crate::config::Config;
use crate::identifiers::IdentifierRegistry;
use std::sync::Arc;

pub struct AppState {
    #[allow(dead_code)]
    pub config: Arc<Config>,
    pub cache: Arc<Cache>,
    pub registry: Arc<IdentifierRegistry>,
    pub task_tx: tokio::sync::mpsc::Sender<crate::worker::TaskCommand>,
}

impl AppState {
    pub fn new(
        config: Config,
        cache: Arc<Cache>,
        registry: Arc<IdentifierRegistry>,
        task_tx: tokio::sync::mpsc::Sender<crate::worker::TaskCommand>,
    ) -> Self {
        Self {
            config: Arc::new(config),
            cache,
            registry,
            task_tx,
        }
    }
}

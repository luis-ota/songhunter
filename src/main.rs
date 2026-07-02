mod audio;
mod cache;
mod config;
mod download;
mod error;
mod http;
mod identifiers;
mod models;
mod state;
mod worker;

use crate::cache::Cache;
use crate::config::Config;
use crate::identifiers::IdentifierRegistry;
use crate::state::AppState;
use crate::worker::run_worker;
use axum::{
    Router,
    routing::{get, post},
};
use std::sync::Arc;
use tokio::sync::mpsc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "songhunter=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::load()?;

    tokio::fs::create_dir_all(&config.temp_dir).await?;
    tokio::fs::create_dir_all("./data").await?;

    let cache = Arc::new(Cache::new("./data/cache.db", config.cache_ttl_hours).await?);
    let registry = Arc::new(IdentifierRegistry::from_config(&config));

    tracing::info!(
        acoustid_configured = config.acoustid.is_some(),
        "config loaded"
    );
    if let Some(ref ac) = config.acoustid {
        tracing::info!(
            key_len = ac.api_key.len(),
            url = %ac.url,
            "acoustid config"
        );
    }
    tracing::info!("enabled providers: {:?}", registry.enabled_backends());
    tracing::info!(
        cookies_file = ?config.ytdlp_cookies_file,
        "cookies config"
    );

    let (tx, rx) = mpsc::channel::<worker::TaskCommand>(config.max_concurrent_tasks * 2);

    let worker_cache = Arc::clone(&cache);
    let worker_registry = Arc::clone(&registry);
    let worker_config = config.clone();
    tokio::spawn(async move {
        run_worker(worker_config, worker_cache, worker_registry, rx).await;
    });

    let state = Arc::new(AppState::new(config.clone(), cache, registry, tx));

    let app = Router::new()
        .route("/api/identify", post(http::identify))
        .route("/api/result/{task_id}", get(http::result))
        .route("/api/health", get(http::health))
        .route("/api/cookies", post(http::save_cookies))
        .route("/api/cookies/status", get(http::cookies_status))
        .route("/auth-insta", get(http::auth_insta_page))
        .fallback_service(tower_http::services::ServeDir::new("static"))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = config.addr();
    tracing::info!("listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

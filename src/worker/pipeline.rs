use crate::audio::AudioProcessor;
use crate::cache::Cache;
use crate::config::Config;
use crate::download::Downloader;
use crate::identifiers::{Identifier, IdentifierRegistry};
use crate::models::{SongMatch, TaskResult, TaskStatus};
use crate::worker::TaskCommand;
use anyhow::Result;
use chrono::Utc;
use futures::future::join_all;
use std::sync::Arc;
use tracing::{error, info, warn};

pub async fn run_worker(
    config: Config,
    cache: Arc<Cache>,
    registry: Arc<IdentifierRegistry>,
    mut rx: tokio::sync::mpsc::Receiver<TaskCommand>,
) {
    let downloader = Arc::new(Downloader::new(&config.ytdlp_path, &config.temp_dir));
    let audio = Arc::new(AudioProcessor::new(
        &config.ffmpeg_path,
        config
            .acoustid
            .as_ref()
            .map(|c| c.fpcalc_path.as_str())
            .unwrap_or("fpcalc"),
        &config.temp_dir,
    ));

    while let Some(cmd) = rx.recv().await {
        let config = config.clone();
        let cache = Arc::clone(&cache);
        let registry = Arc::clone(&registry);
        let downloader = Arc::clone(&downloader);
        let audio = Arc::clone(&audio);

        tokio::spawn(async move {
            if let Err(e) =
                process_task(&config, &cache, &registry, &downloader, &audio, &cmd).await
            {
                error!(task_id = %cmd.task_id, "pipeline error: {}", e);
                let err_result = TaskResult {
                    task_id: cmd.task_id.clone(),
                    status: TaskStatus::Error,
                    progress: 1.0,
                    message: e.to_string(),
                    songs: Vec::new(),
                    completed_at: Some(Utc::now()),
                };
                let _ = cache.set_task(&err_result).await;
            }
        });
    }
}

async fn update_status(
    cache: &Cache,
    task_id: &str,
    status: TaskStatus,
    progress: f32,
    message: &str,
) -> Result<()> {
    cache
        .set_task(&TaskResult {
            task_id: task_id.to_string(),
            status,
            progress,
            message: message.to_string(),
            songs: Vec::new(),
            completed_at: None,
        })
        .await
}

async fn process_task(
    _config: &Config,
    cache: &Cache,
    registry: &IdentifierRegistry,
    downloader: &Downloader,
    audio: &AudioProcessor,
    cmd: &TaskCommand,
) -> Result<()> {
    info!(task_id = %cmd.task_id, "processing task");

    update_status(
        cache,
        &cmd.task_id,
        TaskStatus::Downloading,
        0.1,
        "Baixando áudio do link...",
    )
    .await?;

    let raw_audio = downloader.download(&cmd.url, &cmd.task_id).await?;

    update_status(
        cache,
        &cmd.task_id,
        TaskStatus::Converting,
        0.3,
        "Convertendo áudio...",
    )
    .await?;

    let normalized = audio.normalize(&raw_audio, &cmd.task_id).await?;

    let audio_hash = Cache::audio_hash(&normalized)?;

    if let Some(cached) = cache.get_audio_result(&audio_hash).await? {
        info!(task_id = %cmd.task_id, "cache hit");
        let result = build_result(&cmd.task_id, cached, "Resultado do cache");
        cache.set_task(&result).await?;
        cleanup(audio, downloader, &cmd.task_id).await?;
        return Ok(());
    }

    update_status(
        cache,
        &cmd.task_id,
        TaskStatus::Identifying,
        0.6,
        "Consultando provedores de música...",
    )
    .await?;

    let songs = identify_all(registry, &normalized).await;

    let merged = crate::identifiers::merge_results(songs);

    cache.set_audio_result(&audio_hash, &merged).await?;

    let message = if merged.is_empty() {
        "Nenhuma música identificada.".to_string()
    } else {
        format!("{} música(s) encontrada(s).", merged.len())
    };

    let result = build_result(&cmd.task_id, merged, &message);
    cache.set_task(&result).await?;

    cleanup(audio, downloader, &cmd.task_id).await?;
    Ok(())
}

async fn identify_all(
    registry: &IdentifierRegistry,
    wav_path: &std::path::Path,
) -> Vec<(String, Vec<SongMatch>)> {
    let enabled: Vec<Arc<dyn Identifier>> = registry
        .backends
        .iter()
        .filter(|b| b.enabled())
        .cloned()
        .collect();

    let futures = enabled
        .into_iter()
        .map(|backend| {
            let path = wav_path.to_path_buf();
            async move {
                let name = backend.name().to_string();
                match backend.identify(&path).await {
                    Ok(songs) => (name, songs),
                    Err(e) => {
                        warn!(provider = %name, "identification failed: {}", e);
                        (name, Vec::new())
                    }
                }
            }
        })
        .collect::<Vec<_>>();

    join_all(futures).await
}

fn build_result(task_id: &str, songs: Vec<SongMatch>, message: &str) -> TaskResult {
    TaskResult {
        task_id: task_id.to_string(),
        status: TaskStatus::Done,
        progress: 1.0,
        message: message.to_string(),
        songs,
        completed_at: Some(Utc::now()),
    }
}

async fn cleanup(audio: &AudioProcessor, downloader: &Downloader, task_id: &str) -> Result<()> {
    if let Err(e) = audio.cleanup(task_id).await {
        warn!("failed to cleanup audio files: {}", e);
    }
    if let Err(e) = downloader.cleanup(task_id).await {
        warn!("failed to cleanup download files: {}", e);
    }
    Ok(())
}

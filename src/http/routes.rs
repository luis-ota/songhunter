use crate::error::AppError;
use crate::models::{IdentifyRequest, IdentifyResponse, TaskResult, TaskStatus};
use crate::state::AppState;
use crate::worker::TaskCommand;
use axum::{
    extract::{Path, State},
    response::Json,
};
use std::sync::Arc;
use uuid::Uuid;

pub async fn identify(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<IdentifyRequest>,
) -> Result<Json<IdentifyResponse>, AppError> {
    if payload.url.is_empty() {
        return Err(AppError::InvalidUrl("URL vazia".to_string()));
    }

    let task_id = Uuid::new_v4().to_string();

    let initial = TaskResult {
        task_id: task_id.clone(),
        status: TaskStatus::Queued,
        progress: 0.0,
        message: "Na fila...".to_string(),
        songs: Vec::new(),
        completed_at: None,
    };
    state.cache.set_task(&initial).await?;

    state
        .task_tx
        .send(TaskCommand {
            task_id: task_id.clone(),
            url: payload.url,
        })
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("failed to enqueue task: {e}")))?;

    Ok(Json(IdentifyResponse {
        task_id,
        status: TaskStatus::Queued,
    }))
}

pub async fn result(
    State(state): State<Arc<AppState>>,
    Path(task_id): Path<String>,
) -> Result<Json<TaskResult>, AppError> {
    match state.cache.get_task(&task_id).await? {
        Some(result) => Ok(Json(result)),
        None => Err(AppError::TaskNotFound(task_id)),
    }
}

pub async fn health(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let providers = state.registry.enabled_backends();
    Json(serde_json::json!({
        "status": "ok",
        "providers": providers,
    }))
}

use crate::error::AppError;
use crate::models::{IdentifyRequest, IdentifyResponse, TaskResult, TaskStatus};
use crate::state::AppState;
use crate::worker::TaskCommand;
use axum::{
    extract::{Path, State},
    response::Json,
};
use chrono::{NaiveDateTime, Utc};
use serde::Deserialize;
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

pub async fn auth_insta_page() -> axum::response::Html<String> {
    let html = tokio::fs::read_to_string("static/auth-insta.html")
        .await
        .unwrap_or_else(|_| "<h1>Erro ao carregar página</h1>".to_string());
    axum::response::Html(html)
}

pub async fn health(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let providers = state.registry.enabled_backends();
    Json(serde_json::json!({
        "status": "ok",
        "providers": providers,
    }))
}

#[derive(Deserialize)]
pub struct CookiePayload {
    content: String,
}

pub async fn save_cookies(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CookiePayload>,
) -> Result<Json<serde_json::Value>, AppError> {
    tokio::fs::write(&state.cookies_file, &payload.content)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("failed to write cookies: {e}")))?;
    Ok(Json(serde_json::json!({"status": "ok"})))
}

pub async fn cookies_status(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let content = match tokio::fs::read_to_string(&state.cookies_file).await {
        Ok(c) => c,
        Err(_) => {
            return Json(serde_json::json!({
                "exists": false,
            }));
        }
    };

    if content.trim().is_empty() {
        return Json(serde_json::json!({"exists": false}));
    }

    let mut earliest: Option<NaiveDateTime> = None;
    for line in content.lines() {
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() >= 5 {
            if let Ok(ts) = cols[4].parse::<i64>() {
                if ts == 0 || ts == 2147483647 {
                    continue;
                }
                let dt = chrono::DateTime::from_timestamp(ts, 0).map(|d| d.naive_utc());
                if let Some(dt) = dt {
                    match earliest {
                        None => earliest = Some(dt),
                        Some(e) if dt < e => earliest = Some(dt),
                        _ => {}
                    }
                }
            }
        }
    }

    let now = Utc::now().naive_utc();
    match earliest {
        Some(expires) if expires < now => Json(serde_json::json!({
            "exists": true,
            "expired": true,
            "expired_at": expires.format("%Y-%m-%d %H:%M:%S").to_string(),
        })),
        Some(expires) => {
            let days = (expires - now).num_days();
            Json(serde_json::json!({
                "exists": true,
                "expired": false,
                "days_until_expiry": days,
                "expires_at": expires.format("%Y-%m-%d %H:%M:%S").to_string(),
            }))
        }
        None => Json(serde_json::json!({
            "exists": true,
            "expired": false,
            "days_until_expiry": null,
            "note": "Cookies presentes mas sem data de expiração detectada",
        })),
    }
}

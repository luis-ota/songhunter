use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("download failed: {0}")]
    Download(String),

    #[error("audio conversion failed: {0}")]
    Conversion(String),

    #[error("identification failed: {0}")]
    Identification(String),

    #[error("task not found: {0}")]
    TaskNotFound(String),

    #[error("invalid url: {0}")]
    InvalidUrl(String),

    #[error("internal error: {0}")]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, msg) = match self {
            AppError::TaskNotFound(_) => (StatusCode::NOT_FOUND, self.to_string()),
            AppError::InvalidUrl(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
        };

        let body = Json(json!({ "error": msg }));
        (status, body).into_response()
    }
}

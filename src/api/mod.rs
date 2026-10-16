use crate::db::Database;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json};
use axum::routing::get;
use axum::Router;
use serde_json::json;

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(healthcheck))
        .route(
            "/api/files/check/{hash}",
            get(check_file_get).head(check_file_head),
        )
        .with_state(state)
}

async fn healthcheck() -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(json!({
            "status": "ok",
            "version": env!("CARGO_PKG_VERSION")
        })),
    )
}

async fn check_file_head(
    State(state): State<AppState>,
    Path(hash): Path<String>,
) -> impl IntoResponse {
    match state.db.get_file_by_hash(&hash) {
        Ok(Some(_)) => StatusCode::OK,
        _ => StatusCode::NOT_FOUND,
    }
}

async fn check_file_get(
    State(state): State<AppState>,
    Path(hash): Path<String>,
) -> impl IntoResponse {
    match state.db.get_file_by_hash(&hash) {
        Ok(Some(record)) => (
            StatusCode::OK,
            Json(json!({
                "exists": true,
                "file_name": record.file_name,
                "file_size": record.file_size,
                "sha256": record.sha256,
                "mime_type": record.mime_type,
                "storage_path": record.storage_path,
                "created_at": record.created_at,
                "mtime": record.mtime,
                "analysis_status": record.analysis_status,
            })),
        )
            .into_response(),
        _ => (
            StatusCode::NOT_FOUND,
            Json(json!({
                "exists": false,
                "error": "File not found"
            })),
        )
            .into_response(),
    }
}

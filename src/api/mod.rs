use crate::config::Config;
use crate::db::{Database, ShareRecord};
use crate::fuxico::{FuxicoClient, FuxicoShareRequest};
use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::header::{CONTENT_DISPOSITION, CONTENT_LENGTH, CONTENT_TYPE};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Json, Response};
use axum::routing::{delete, get, post};
use axum::Router;
use serde::Deserialize;
use serde_json::json;
use sha2::Digest;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio_util::io::ReaderStream;

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub config: Config,
    pub fuxico: Arc<FuxicoClient>,
    pub vision: Arc<dyn crate::vision::VisionEmbedder>,
}

#[derive(Debug, Deserialize)]
pub struct CreateSharePayload {
    pub storage_path: String,
    pub ttl: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(crate::web_ui::web_ui_handler))
        .route("/ui", get(crate::web_ui::web_ui_handler))
        .route("/api/files", get(crate::web_ui::list_files_handler))
        .route("/raw/{*path}", get(crate::web_ui::raw_file_handler))
        .route("/s3/{bucket}", get(crate::s3::s3_list_objects_v2_handler))
        .route(
            "/s3/{bucket}/{*key}",
            get(crate::s3::s3_get_object_handler)
                .put(crate::s3::s3_put_object_handler)
                .delete(crate::s3::s3_delete_object_handler),
        )
        .route("/health", get(healthcheck))
        .route("/api/search", get(search_handler))
        .route(
            "/api/files/check/{hash}",
            get(check_file_get).head(check_file_head),
        )
        .route("/api/shares", post(create_share_handler))
        .route("/api/shares/{token}", delete(revoke_share_handler))
        .route("/api/internal/stream/{token}", get(stream_share_handler))
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
        ),
        _ => (
            StatusCode::NOT_FOUND,
            Json(json!({
                "exists": false,
                "error": "Arquivo não encontrado para este hash SHA-256"
            })),
        ),
    }
}

async fn search_handler(
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let q = match query.q {
        Some(s) if !s.trim().is_empty() => s.trim().to_string(),
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "Parâmetro de busca 'q' é obrigatório" })),
            ));
        }
    };

    let limit = query.limit.unwrap_or(20).clamp(1, 100);
    let offset = query.offset.unwrap_or(0);

    // 1. Busca FTS5
    let (_fts_total, fts_results) = state.db.search_fts(&q, 100, 0).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("Erro na busca FTS5: {}", e) })),
        )
    })?;

    // 2. Busca Semântica Vetorial de Visão
    let vec_matches = match state.vision.embed_text(&q) {
        Ok(q_vec) => state.db.search_vec(&q_vec, 100).unwrap_or_default(),
        Err(_) => Vec::new(),
    };

    // 3. Reciprocal Rank Fusion (RRF)
    const K: f64 = 60.0;
    use std::collections::HashMap;

    struct RrfEntry {
        result: crate::db::SearchResult,
        rrf_score: f64,
    }

    let mut map: HashMap<i64, RrfEntry> = HashMap::new();

    for (rank, mut fts_res) in fts_results.into_iter().enumerate() {
        let score = 1.0 / (K + (rank as f64) + 1.0);
        let id = fts_res.id;
        fts_res.match_type = "text".to_string();
        map.insert(
            id,
            RrfEntry {
                result: fts_res,
                rrf_score: score,
            },
        );
    }

    for (rank, (file_id, cos_score)) in vec_matches.into_iter().enumerate() {
        let score = 1.0 / (K + (rank as f64) + 1.0);
        if let Some(entry) = map.get_mut(&file_id) {
            entry.rrf_score += score;
            entry.result.match_type = "hybrid".to_string();
        } else if let Ok(Some(file_rec)) = state.db.get_file_by_id(file_id) {
            let clean_path = file_rec.storage_path.trim_start_matches('/').to_string();
            let res = crate::db::SearchResult {
                id: file_id,
                storage_path: clean_path,
                file_name: file_rec.file_name,
                file_size: file_rec.file_size,
                sha256: file_rec.sha256,
                mime_type: file_rec.mime_type,
                snippet: format!("[Similaridade visual: {:.2}]", cos_score),
                score: cos_score,
                match_type: "semantic_image".to_string(),
            };
            map.insert(
                file_id,
                RrfEntry {
                    result: res,
                    rrf_score: score,
                },
            );
        }
    }

    let mut combined: Vec<crate::db::SearchResult> = map
        .into_values()
        .map(|mut entry| {
            entry.result.score = entry.rrf_score;
            entry.result
        })
        .collect();

    combined.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let total = combined.len();
    let paged: Vec<crate::db::SearchResult> =
        combined.into_iter().skip(offset).take(limit).collect();

    Ok((
        StatusCode::OK,
        Json(json!({
            "query": q,
            "total": total,
            "results": paged
        })),
    ))
}

async fn create_share_handler(
    State(state): State<AppState>,
    Json(payload): Json<CreateSharePayload>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let file = state
        .db
        .get_file_by_storage_path(&payload.storage_path)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "Arquivo não encontrado no storage do Mocambo" })),
            )
        })?;

    let now_nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let now_secs = (now_nanos / 1_000_000_000) as i64;

    let mut hasher = sha2::Sha256::new();
    hasher.update(now_nanos.to_le_bytes());
    hasher.update(payload.storage_path.as_bytes());
    let hash = hex::encode(hasher.finalize());
    let token = format!("moca_tok_{}", &hash[..32]);

    let stream_url = format!(
        "{}/api/internal/stream/{}",
        state.config.mocambo_internal_url.trim_end_matches('/'),
        token
    );

    let fux_req = FuxicoShareRequest {
        file_name: file.file_name.clone(),
        file_size: file.file_size,
        mime_type: file.mime_type.clone(),
        mocambo_token: token.clone(),
        mocambo_stream_url: stream_url,
        ttl: payload.ttl,
    };

    let fux_resp = state.fuxico.create_share(&fux_req).await.map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            Json(json!({ "error": format!("Falha ao registrar share no Fuxico: {}", e) })),
        )
    })?;

    let share_record = ShareRecord {
        id: format!("share_{}", &hash[..16]),
        token: token.clone(),
        user_id: file.user_id,
        storage_path: file.storage_path.clone(),
        fuxico_slug: fux_resp.slug.clone(),
        fuxico_url: fux_resp.url.clone(),
        created_at: now_secs,
        expires_at: None,
        status: "active".to_string(),
    };

    state.db.create_share(&share_record).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
    })?;

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "token": token,
            "storage_path": file.storage_path,
            "fuxico_slug": fux_resp.slug,
            "fuxico_url": fux_resp.url,
            "created_at": now_secs
        })),
    ))
}

async fn revoke_share_handler(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let share = state
        .db
        .get_share_by_token(&token)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "Compartilhamento não encontrado" })),
            )
        })?;

    let _ = state.fuxico.revoke_share(&share.fuxico_slug).await;

    state.db.revoke_share(&token).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
    })?;

    Ok((
        StatusCode::OK,
        Json(json!({
            "success": true,
            "token": token
        })),
    ))
}

async fn stream_share_handler(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Result<Response, (StatusCode, &'static str)> {
    let share = state
        .db
        .get_share_by_token(&token)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?
        .ok_or((StatusCode::NOT_FOUND, "Share not found"))?;

    if share.status != "active" {
        return Err((StatusCode::GONE, "Share revoked or expired"));
    }

    let now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    if let Some(exp) = share.expires_at {
        if exp <= now_secs {
            let _ = state.db.revoke_share(&token);
            return Err((StatusCode::GONE, "Share expired"));
        }
    }

    let abs_path = state.config.storage_dir.join(&share.storage_path);
    if !abs_path.exists() {
        return Err((StatusCode::NOT_FOUND, "Physical file not found"));
    }

    let file = tokio::fs::File::open(&abs_path)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Cannot open file"))?;
    let metadata = file.metadata().await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Cannot read file metadata",
        )
    })?;

    let mime = mime_guess::from_path(&abs_path)
        .first_or_octet_stream()
        .to_string();

    let filename = abs_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("file");

    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    let mut headers = HeaderMap::new();
    if let Ok(v) = HeaderValue::from_str(&mime) {
        headers.insert(CONTENT_TYPE, v);
    }
    if let Ok(v) = HeaderValue::from_str(&metadata.len().to_string()) {
        headers.insert(CONTENT_LENGTH, v);
    }
    let disposition = format!("inline; filename=\"{}\"", filename);
    if let Ok(v) = HeaderValue::from_str(&disposition) {
        headers.insert(CONTENT_DISPOSITION, v);
    }

    Ok((headers, body).into_response())
}

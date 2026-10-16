use axum::body::{to_bytes, Body};
use axum::extract::{Path, Query, State};
use axum::http::header::{CONTENT_LENGTH, CONTENT_TYPE, ETAG};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path as StdPath, PathBuf};
use tokio_util::io::ReaderStream;

use crate::api::AppState;

#[derive(Debug, Deserialize)]
pub struct ListObjectsQuery {
    pub prefix: Option<String>,
    #[serde(rename = "max-keys")]
    pub max_keys: Option<usize>,
}

/// PUT /s3/{bucket}/{*key} — PutObject S3 compatível
pub async fn s3_put_object_handler(
    State(state): State<AppState>,
    Path((bucket, key)): Path<(String, String)>,
    body: Body,
) -> Response {
    if bucket.contains("..") || key.contains("..") {
        return (StatusCode::BAD_REQUEST, "Caminho inválido").into_response();
    }

    let clean_key = key.trim_start_matches('/');
    let target_dir = state.config.storage_dir.join(&bucket);
    let target_file = target_dir.join(clean_key);

    if let Some(parent) = target_file.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Falha ao criar diretórios: {e}"),
            )
                .into_response();
        }
    }

    let bytes = match to_bytes(body, 256 * 1024 * 1024).await {
        Ok(b) => b,
        Err(e) => {
            return (StatusCode::BAD_REQUEST, format!("Falha ao ler corpo: {e}")).into_response()
        }
    };

    if let Err(e) = fs::write(&target_file, &bytes) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Falha ao gravar arquivo: {e}"),
        )
            .into_response();
    }

    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let hash = hex::encode(hasher.finalize());
    let etag_val = format!("\"{}\"", hash);

    let mime = mime_guess::from_path(&target_file)
        .first_or_octet_stream()
        .to_string();

    let storage_path = format!("/{}/{}", bucket, clean_key);
    let file_name = target_file
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_else(|| clean_key.to_string());

    let _ = state.db.record_file(
        "s3_user",
        &storage_path,
        &file_name,
        bytes.len() as i64,
        &hash,
        &mime,
    );

    let mut headers = HeaderMap::new();
    if let Ok(v) = HeaderValue::from_str(&etag_val) {
        headers.insert(ETAG, v);
    }
    headers.insert(CONTENT_LENGTH, HeaderValue::from(0));

    (StatusCode::OK, headers, ()).into_response()
}

/// GET /s3/{bucket}/{*key} — GetObject S3 compatível
pub async fn s3_get_object_handler(
    State(state): State<AppState>,
    Path((bucket, key)): Path<(String, String)>,
) -> Response {
    if bucket.contains("..") || key.contains("..") {
        return (StatusCode::BAD_REQUEST, "Caminho inválido").into_response();
    }

    let clean_key = key.trim_start_matches('/');
    let target_file = state.config.storage_dir.join(&bucket).join(clean_key);

    let canon_root = match state.config.storage_dir.canonicalize() {
        Ok(r) => r,
        Err(_) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, "Configuração inválida").into_response()
        }
    };

    let canon_file = match target_file.canonicalize() {
        Ok(f) => f,
        Err(_) => {
            return (
                StatusCode::NOT_FOUND,
                s3_error_xml("NoSuchKey", "The specified key does not exist."),
            )
                .into_response()
        }
    };

    if !canon_file.starts_with(&canon_root) || !canon_file.is_file() {
        return (
            StatusCode::NOT_FOUND,
            s3_error_xml("NoSuchKey", "The specified key does not exist."),
        )
            .into_response();
    }

    let mime = mime_guess::from_path(&canon_file)
        .first_or_octet_stream()
        .to_string();

    let file = match tokio::fs::File::open(&canon_file).await {
        Ok(f) => f,
        Err(_) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, "Erro ao abrir arquivo").into_response()
        }
    };

    let len = match file.metadata().await {
        Ok(m) => m.len(),
        Err(_) => 0,
    };

    let storage_path = format!("/{}/{}", bucket, clean_key);
    let hash = state
        .db
        .get_file_by_storage_path(&storage_path)
        .ok()
        .flatten()
        .map(|r| r.sha256)
        .unwrap_or_else(|| {
            let bytes = fs::read(&canon_file).unwrap_or_default();
            let mut hasher = Sha256::new();
            hasher.update(&bytes);
            hex::encode(hasher.finalize())
        });

    let etag_val = format!("\"{}\"", hash);
    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    let mut headers = HeaderMap::new();
    if let Ok(val) = HeaderValue::from_str(&mime) {
        headers.insert(CONTENT_TYPE, val);
    }
    if let Ok(v) = HeaderValue::from_str(&etag_val) {
        headers.insert(ETAG, v);
    }
    headers.insert(CONTENT_LENGTH, HeaderValue::from(len));

    (StatusCode::OK, headers, body).into_response()
}

/// DELETE /s3/{bucket}/{*key} — DeleteObject S3 compatível
pub async fn s3_delete_object_handler(
    State(state): State<AppState>,
    Path((bucket, key)): Path<(String, String)>,
) -> Response {
    if bucket.contains("..") || key.contains("..") {
        return (StatusCode::BAD_REQUEST, "Caminho inválido").into_response();
    }

    let clean_key = key.trim_start_matches('/');
    let target_file = state.config.storage_dir.join(&bucket).join(clean_key);

    if target_file.exists() && target_file.is_file() {
        let _ = fs::remove_file(&target_file);
    }

    let storage_path = format!("/{}/{}", bucket, clean_key);
    let _ = state.db.remove_file_by_path(&storage_path);

    StatusCode::NO_CONTENT.into_response()
}

/// GET /s3/{bucket} — ListObjectsV2 S3 compatível
pub async fn s3_list_objects_v2_handler(
    State(state): State<AppState>,
    Path(bucket): Path<String>,
    Query(query): Query<ListObjectsQuery>,
) -> Response {
    if bucket.contains("..") {
        return (StatusCode::BAD_REQUEST, "Bucket inválido").into_response();
    }

    let bucket_dir = state.config.storage_dir.join(&bucket);
    let prefix = query.prefix.unwrap_or_default();
    let max_keys = query.max_keys.unwrap_or(1000);

    let mut contents_xml = String::new();
    let mut key_count = 0;

    if bucket_dir.exists() && bucket_dir.is_dir() {
        let files = collect_files_recursively(&bucket_dir, "");
        for (rel_key, full_path) in files {
            if !prefix.is_empty() && !rel_key.starts_with(&prefix) {
                continue;
            }

            let meta = match full_path.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };

            let size = meta.len();
            let storage_path = format!("/{}/{}", bucket, rel_key);
            let hash = state
                .db
                .get_file_by_storage_path(&storage_path)
                .ok()
                .flatten()
                .map(|r| r.sha256)
                .unwrap_or_default();

            contents_xml.push_str(&format!(
                "  <Contents>\n    <Key>{}</Key>\n    <LastModified>2026-10-16T20:00:00.000Z</LastModified>\n    <ETag>&quot;{}&quot;</ETag>\n    <Size>{}</Size>\n    <StorageClass>STANDARD</StorageClass>\n  </Contents>\n",
                escape_xml(&rel_key),
                hash,
                size
            ));

            key_count += 1;
            if key_count >= max_keys {
                break;
            }
        }
    }

    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<ListBucketResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
  <Name>{}</Name>
  <Prefix>{}</Prefix>
  <KeyCount>{}</KeyCount>
  <MaxKeys>{}</MaxKeys>
  <IsTruncated>false</IsTruncated>
{}</ListBucketResult>"#,
        escape_xml(&bucket),
        escape_xml(&prefix),
        key_count,
        max_keys,
        contents_xml
    );

    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/xml; charset=utf-8"),
    );

    (StatusCode::OK, headers, xml).into_response()
}

fn collect_files_recursively(dir: &StdPath, base_rel: &str) -> Vec<(String, PathBuf)> {
    let mut result = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                continue;
            }

            let rel_key = if base_rel.is_empty() {
                name
            } else {
                format!("{}/{}", base_rel, name)
            };

            if path.is_dir() {
                result.extend(collect_files_recursively(&path, &rel_key));
            } else if path.is_file() {
                result.push((rel_key, path));
            }
        }
    }
    result
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn s3_error_xml(code: &str, message: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<Error>
  <Code>{}</Code>
  <Message>{}</Message>
</Error>"#,
        code, message
    )
}

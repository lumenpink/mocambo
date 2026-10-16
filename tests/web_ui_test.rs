use axum::body::Body;
use axum::http::{Request, StatusCode};
use mocambo::app::create_app;
use mocambo::config::Config;
use mocambo::db::Database;
use std::fs;
use tempfile::TempDir;
use tower::ServiceExt;

#[tokio::test]
async fn test_web_ui_html_endpoint() {
    let temp_dir = TempDir::new().unwrap();
    let config = Config::new_for_test(temp_dir.path());
    let db = Database::init(&config.db_path).unwrap();
    let app = create_app(config, db).await.unwrap();

    // GET /
    let res = app
        .clone()
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let content_type = res
        .headers()
        .get("content-type")
        .expect("content-type header")
        .to_str()
        .unwrap();
    assert!(content_type.contains("text/html"));

    let csp = res
        .headers()
        .get("content-security-policy")
        .expect("CSP header presente")
        .to_str()
        .unwrap();
    assert!(csp.contains("default-src"));

    let body = axum::body::to_bytes(res.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let body_str = String::from_utf8_lossy(&body);
    assert!(body_str.contains("Mocambo"));
    assert!(body_str.contains("id=\"search-input\""));
    assert!(body_str.contains("id=\"file-grid\""));

    // GET /ui
    let res_ui = app
        .oneshot(Request::builder().uri("/ui").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(res_ui.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_api_files_and_raw_streaming() {
    let temp_dir = TempDir::new().unwrap();
    let config = Config::new_for_test(temp_dir.path());
    let db = Database::init(&config.db_path).unwrap();

    // Criar arquivos e pastas de teste
    let subfolder = config.storage_dir.join("documentos");
    fs::create_dir_all(&subfolder).unwrap();
    fs::write(
        config.storage_dir.join("manifesto.txt"),
        b"Soberania de dados e privacidade",
    )
    .unwrap();
    fs::write(subfolder.join("relatorio.pdf"), b"%PDF-1.4 mock content").unwrap();

    let app = create_app(config, db).await.unwrap();

    // GET /api/files?path=/
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/files?path=/")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json.is_array());
    let list = json.as_array().unwrap();
    assert!(list
        .iter()
        .any(|item| item["name"] == "documentos" && item["is_dir"] == true));
    assert!(list
        .iter()
        .any(|item| item["name"] == "manifesto.txt" && item["is_dir"] == false));

    // GET /raw/manifesto.txt
    let res_raw = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/raw/manifesto.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res_raw.status(), StatusCode::OK);
    let raw_bytes = axum::body::to_bytes(res_raw.into_body(), 1024 * 1024)
        .await
        .unwrap();
    assert_eq!(raw_bytes.as_ref(), b"Soberania de dados e privacidade");

    // Path traversal block test
    let res_traversal = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/raw/../etc/passwd")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(
        res_traversal.status() == StatusCode::BAD_REQUEST
            || res_traversal.status() == StatusCode::FORBIDDEN
            || res_traversal.status() == StatusCode::NOT_FOUND
    );

    // Mutação: Pasta requisitada via raw streaming deve ser rejeitada com 403 Forbidden
    let res_dir_raw = app
        .oneshot(
            Request::builder()
                .uri("/raw/documentos")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res_dir_raw.status(), StatusCode::FORBIDDEN);
}

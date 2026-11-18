use axum::body::Body;
use axum::http::{Request, StatusCode};
use mocambo::app::create_app;
use mocambo::config::Config;
use mocambo::db::Database;
use serde_json::json;
use tempfile::TempDir;
use tower::ServiceExt;

#[tokio::test]
async fn test_webdav_put_with_remote_user_header() {
    let temp_dir = TempDir::new().unwrap();
    let config = Config::new_for_test(temp_dir.path());
    std::fs::create_dir_all(config.storage_dir.join("zotero")).unwrap();
    let db = Database::init(&config.db_path).unwrap();
    let app = create_app(config.clone(), db.clone()).await.unwrap();

    let content = b"%PDF-1.4 test paper content for zotero sync with authelia auth";
    let req = Request::builder()
        .method("PUT")
        .uri("/dav/zotero/paper_lumen.pdf")
        .header("Remote-User", "lumen")
        .body(Body::from(content.as_slice()))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert!(
        res.status().is_success(),
        "PUT deve retornar 2xx, retornou: {}",
        res.status()
    );

    // Buscar no banco e verificar se o user_id é "lumen"
    let file = db
        .get_file_by_storage_path("/dav/zotero/paper_lumen.pdf")
        .unwrap()
        .expect("arquivo deve estar indexado");

    assert_eq!(file.user_id, "lumen");
    assert_eq!(file.file_name, "paper_lumen.pdf");
}

#[tokio::test]
async fn test_webdav_put_with_x_forwarded_user_header() {
    let temp_dir = TempDir::new().unwrap();
    let config = Config::new_for_test(temp_dir.path());
    std::fs::create_dir_all(config.storage_dir.join("zotero")).unwrap();
    let db = Database::init(&config.db_path).unwrap();
    let app = create_app(config.clone(), db.clone()).await.unwrap();

    let content = b"%PDF-1.4 forwarded user test";
    let req = Request::builder()
        .method("PUT")
        .uri("/dav/zotero/paper_maria.pdf")
        .header("X-Forwarded-User", "maria")
        .body(Body::from(content.as_slice()))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert!(res.status().is_success());

    let file = db
        .get_file_by_storage_path("/dav/zotero/paper_maria.pdf")
        .unwrap()
        .expect("arquivo deve estar indexado");

    assert_eq!(file.user_id, "maria");
}

#[tokio::test]
async fn test_webdav_put_without_user_header_fallback() {
    let temp_dir = TempDir::new().unwrap();
    let config = Config::new_for_test(temp_dir.path());
    std::fs::create_dir_all(config.storage_dir.join("zotero")).unwrap();
    let db = Database::init(&config.db_path).unwrap();
    let app = create_app(config.clone(), db.clone()).await.unwrap();

    let content = b"unauthenticated local test fallback content";
    let req = Request::builder()
        .method("PUT")
        .uri("/dav/zotero/paper_default.pdf")
        .body(Body::from(content.as_slice()))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert!(res.status().is_success());

    let file = db
        .get_file_by_storage_path("/dav/zotero/paper_default.pdf")
        .unwrap()
        .expect("arquivo deve estar indexado");

    assert_eq!(file.user_id, "default");
}

#[tokio::test]
async fn test_api_dedup_with_remote_user_header() {
    let temp_dir = TempDir::new().unwrap();
    let config = Config::new_for_test(temp_dir.path());
    std::fs::create_dir_all(config.storage_dir.join("zotero")).unwrap();
    let db = Database::init(&config.db_path).unwrap();
    let app = create_app(config.clone(), db.clone()).await.unwrap();

    // 1. Grava um arquivo inicial com lumen
    let content = b"conteudo compartilhado para teste de dedup com auth";
    let req = Request::builder()
        .method("PUT")
        .uri("/dav/zotero/original.pdf")
        .header("Remote-User", "lumen")
        .body(Body::from(content.as_slice()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert!(res.status().is_success());

    let original = db
        .get_file_by_storage_path("/dav/zotero/original.pdf")
        .unwrap()
        .expect("original encontrado");

    // 2. Outro usuário dedublica informando Remote-User
    let dedup_payload = json!({
        "sha256": original.sha256,
        "storage_path": "/dav/Drive/deduped.pdf",
        "file_name": "deduped.pdf",
        "source": "Zotero",
        "original_path": "Zotero/storage/XYZ/deduped.pdf"
    });

    let dedup_req = Request::builder()
        .method("POST")
        .uri("/api/files/dedup")
        .header("content-type", "application/json")
        .header("Remote-User", "maria")
        .body(Body::from(serde_json::to_vec(&dedup_payload).unwrap()))
        .unwrap();

    let dedup_res = app.clone().oneshot(dedup_req).await.unwrap();
    assert_eq!(dedup_res.status(), StatusCode::CREATED);

    let deduped = db
        .get_file_by_storage_path("/dav/Drive/deduped.pdf")
        .unwrap()
        .expect("arquivo dedup deve existir no banco");

    assert_eq!(deduped.user_id, "maria");
}

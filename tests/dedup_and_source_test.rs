use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use mocambo::app::create_app;
use mocambo::config::Config;
use mocambo::db::Database;
use serde_json::Value;
use std::os::unix::fs::MetadataExt;
use tempfile::TempDir;
use tower::ServiceExt;

#[tokio::test]
async fn test_db_record_and_query_with_source() {
    let temp_dir = TempDir::new().unwrap();
    let config = Config::new_for_test(temp_dir.path());
    let db = Database::init(&config.db_path).unwrap();

    let hash = "1111222233334444555566667777888899990000aaaabbbbccccddddeeeeffff";
    db.record_file_with_metadata(
        "default",
        "/dav/Timeline/2026/11/photo.jpg",
        "photo.jpg",
        1024,
        hash,
        "image/jpeg",
        Some("WhatsApp"),
        Some("WhatsApp/WhatsApp Images/photo.jpg"),
    )
    .unwrap();

    let record = db.get_file_by_hash(hash).unwrap().expect("record found");
    assert_eq!(record.source.as_deref(), Some("WhatsApp"));
    assert_eq!(
        record.original_path.as_deref(),
        Some("WhatsApp/WhatsApp Images/photo.jpg")
    );

    let sources = db.get_sources().unwrap();
    assert!(sources.contains(&"WhatsApp".to_string()));

    let files_by_source = db.get_files_by_source("WhatsApp").unwrap();
    assert_eq!(files_by_source.len(), 1);
    assert_eq!(files_by_source[0].file_name, "photo.jpg");
}

#[tokio::test]
async fn test_api_check_hash_returns_source_metadata() {
    let temp_dir = TempDir::new().unwrap();
    let config = Config::new_for_test(temp_dir.path());
    let db = Database::init(&config.db_path).unwrap();

    let hash = "222233334444555566667777888899990000aaaabbbbccccddddeeeeffff1111";
    db.record_file_with_metadata(
        "default",
        "/dav/Timeline/2026/11/camera.jpg",
        "camera.jpg",
        2048,
        hash,
        "image/jpeg",
        Some("Camera"),
        Some("DCIM/Camera/camera.jpg"),
    )
    .unwrap();

    let app = create_app(config.clone(), db.clone()).await.unwrap();

    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/files/check/{}", hash))
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["exists"], true);
    assert_eq!(json["source"], "Camera");
    assert_eq!(json["original_path"], "DCIM/Camera/camera.jpg");
}

#[tokio::test]
async fn test_api_dedup_endpoint_creates_hardlink() {
    let temp_dir = TempDir::new().unwrap();
    let config = Config::new_for_test(temp_dir.path());
    let db = Database::init(&config.db_path).unwrap();

    // Prepare an original file on disk
    let original_rel = "Timeline/2026/11/original.jpg";
    let original_abs = config.storage_dir.join(original_rel);
    std::fs::create_dir_all(original_abs.parent().unwrap()).unwrap();
    std::fs::write(&original_abs, b"sample duplicate photo content").unwrap();

    let hash = "aabbccddeeff00112233445566778899aabbccddeeff00112233445566778899";
    db.record_file_with_metadata(
        "default",
        "/dav/Timeline/2026/11/original.jpg",
        "original.jpg",
        30,
        hash,
        "image/jpeg",
        Some("Camera"),
        Some("DCIM/Camera/original.jpg"),
    )
    .unwrap();

    let app = create_app(config.clone(), db.clone()).await.unwrap();

    // Dedup link payload for duplicate file
    let dedup_payload = serde_json::json!({
        "sha256": hash,
        "storage_path": "/dav/Timeline/2026/11/WhatsApp/duplicate.jpg",
        "file_name": "duplicate.jpg",
        "source": "WhatsApp",
        "original_path": "Android/media/com.whatsapp/duplicate.jpg"
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/files/dedup")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&dedup_payload).unwrap()))
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);

    let target_abs = config
        .storage_dir
        .join("Timeline/2026/11/WhatsApp/duplicate.jpg");
    assert!(target_abs.exists());

    // Check POSIX hardlink inode equality
    let orig_ino = std::fs::metadata(&original_abs).unwrap().ino();
    let target_ino = std::fs::metadata(&target_abs).unwrap().ino();
    assert_eq!(orig_ino, target_ino);

    // Verify DB entry
    let dup_rec = db
        .get_file_by_storage_path("/dav/Timeline/2026/11/WhatsApp/duplicate.jpg")
        .unwrap()
        .expect("dup record found");
    assert_eq!(dup_rec.source.as_deref(), Some("WhatsApp"));
    assert_eq!(dup_rec.sha256, hash);
}

#[tokio::test]
async fn test_webdav_put_with_source_headers_and_dedup() {
    let temp_dir = TempDir::new().unwrap();
    let config = Config::new_for_test(temp_dir.path());
    let db = Database::init(&config.db_path).unwrap();
    let app = create_app(config.clone(), db.clone()).await.unwrap();

    let content = b"unique-image-payload-for-webdav-sync";
    std::fs::create_dir_all(config.storage_dir.join("Timeline/2026/11")).unwrap();

    // 1. Upload original file via WebDAV PUT
    let req1 = Request::builder()
        .method("PUT")
        .uri("/dav/Timeline/2026/11/camera.jpg")
        .header("X-Mocambo-Source", "Camera")
        .header("X-Mocambo-Original-Path", "DCIM/Camera/camera.jpg")
        .body(Body::from(content.to_vec()))
        .unwrap();

    let res1 = app.clone().oneshot(req1).await.unwrap();
    assert!(res1.status().is_success());

    let rec1 = db
        .get_file_by_storage_path("/dav/Timeline/2026/11/camera.jpg")
        .unwrap()
        .expect("rec1 found");
    assert_eq!(rec1.source.as_deref(), Some("Camera"));
    assert_eq!(
        rec1.original_path.as_deref(),
        Some("DCIM/Camera/camera.jpg")
    );

    // 2. Upload duplicate file with same content via WebDAV PUT with WhatsApp headers
    let req2 = Request::builder()
        .method("PUT")
        .uri("/dav/Timeline/2026/11/whatsapp.jpg")
        .header("X-Mocambo-Source", "WhatsApp")
        .header("X-Mocambo-Original-Path", "WhatsApp/whatsapp.jpg")
        .body(Body::from(content.to_vec()))
        .unwrap();

    let res2 = app.clone().oneshot(req2).await.unwrap();
    assert!(res2.status().is_success());

    let rec2 = db
        .get_file_by_storage_path("/dav/Timeline/2026/11/whatsapp.jpg")
        .unwrap()
        .expect("rec2 found");
    assert_eq!(rec2.source.as_deref(), Some("WhatsApp"));
    assert_eq!(rec2.sha256, rec1.sha256);

    // Verify storage hardlink deduplication: same inode on disk
    let path1 = config.storage_dir.join("Timeline/2026/11/camera.jpg");
    let path2 = config.storage_dir.join("Timeline/2026/11/whatsapp.jpg");
    assert!(path1.exists());
    assert!(path2.exists());
    assert_eq!(
        std::fs::metadata(&path1).unwrap().ino(),
        std::fs::metadata(&path2).unwrap().ino()
    );
}

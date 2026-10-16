use axum::body::Body;
use axum::http::{Request, StatusCode};
use mocambo::app::create_app;
use mocambo::config::Config;
use mocambo::db::Database;
use tempfile::TempDir;
use tower::ServiceExt; // for oneshot

#[tokio::test]
async fn test_healthcheck() {
    let temp_dir = TempDir::new().unwrap();
    let config = Config::new_for_test(temp_dir.path());
    let db = Database::init(&config.db_path).unwrap();
    let app = create_app(config, db).await.unwrap();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_check_hash_not_found() {
    let temp_dir = TempDir::new().unwrap();
    let config = Config::new_for_test(temp_dir.path());
    let db = Database::init(&config.db_path).unwrap();
    let app = create_app(config, db).await.unwrap();

    let fake_hash = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
    let response = app
        .oneshot(
            Request::builder()
                .method("HEAD")
                .uri(format!("/api/files/check/{}", fake_hash))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_webdav_put_and_check_hash() {
    let temp_dir = TempDir::new().unwrap();
    let config = Config::new_for_test(temp_dir.path());
    let db = Database::init(&config.db_path).unwrap();
    let app = create_app(config.clone(), db.clone()).await.unwrap();

    let file_content = b"Hello, Mocambo sovereign storage!";
    // SHA256 of "Hello, Mocambo sovereign storage!"
    // echo -n "Hello, Mocambo sovereign storage!" | sha256sum
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(file_content);
    let expected_hash = hex::encode(hasher.finalize());

    // 1. Upload via WebDAV PUT
    let put_req = Request::builder()
        .method("PUT")
        .uri("/dav/Drive/hello.txt")
        .body(Body::from(file_content.to_vec()))
        .unwrap();

    let put_res = app.clone().oneshot(put_req).await.unwrap();
    assert!(
        put_res.status() == StatusCode::CREATED
            || put_res.status() == StatusCode::NO_CONTENT
            || put_res.status() == StatusCode::OK,
        "PUT failed with status: {}",
        put_res.status()
    );

    // 2. Verify physical POSIX persistence on disk
    let expected_file_path = config.storage_dir.join("Drive").join("hello.txt");
    assert!(
        expected_file_path.exists(),
        "File was not written to POSIX disk at {:?}",
        expected_file_path
    );
    let read_back = std::fs::read(&expected_file_path).unwrap();
    assert_eq!(read_back, file_content);

    // 3. Verify SQLite catalog record
    let record = db.get_file_by_hash(&expected_hash).unwrap();
    assert!(
        record.is_some(),
        "File record was not found in database by hash"
    );
    let record = record.unwrap();
    assert_eq!(record.file_name, "hello.txt");
    assert_eq!(record.file_size, file_content.len() as i64);
    assert_eq!(record.sha256, expected_hash);

    // 4. Verify HEAD /api/files/check/{hash} returns 200 OK
    let head_req = Request::builder()
        .method("HEAD")
        .uri(format!("/api/files/check/{}", expected_hash))
        .body(Body::empty())
        .unwrap();
    let head_res = app.clone().oneshot(head_req).await.unwrap();
    assert_eq!(head_res.status(), StatusCode::OK);

    // 5. Verify GET /api/files/check/{hash} returns metadata JSON
    let get_req = Request::builder()
        .method("GET")
        .uri(format!("/api/files/check/{}", expected_hash))
        .body(Body::empty())
        .unwrap();
    let get_res = app.oneshot(get_req).await.unwrap();
    assert_eq!(get_res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_webdav_overwrite_updates_hash_and_size() {
    let temp_dir = TempDir::new().unwrap();
    let config = Config::new_for_test(temp_dir.path());
    let db = Database::init(&config.db_path).unwrap();
    let app = create_app(config.clone(), db.clone()).await.unwrap();

    use sha2::{Digest, Sha256};
    let content_v1 = b"Version 1 content";
    let hash_v1 = hex::encode(Sha256::digest(content_v1));

    let content_v2 = b"Version 2 updated longer content for mutation verification";
    let hash_v2 = hex::encode(Sha256::digest(content_v2));

    // Upload V1
    let put_req1 = Request::builder()
        .method("PUT")
        .uri("/dav/Drive/doc.txt")
        .body(Body::from(content_v1.to_vec()))
        .unwrap();
    let res1 = app.clone().oneshot(put_req1).await.unwrap();
    assert!(res1.status().is_success());

    assert!(db.get_file_by_hash(&hash_v1).unwrap().is_some());

    // Upload V2 (Overwrite)
    let put_req2 = Request::builder()
        .method("PUT")
        .uri("/dav/Drive/doc.txt")
        .body(Body::from(content_v2.to_vec()))
        .unwrap();
    let res2 = app.clone().oneshot(put_req2).await.unwrap();
    assert!(res2.status().is_success());

    // Verify V2 is active and V1 is replaced
    let record_v2 = db.get_file_by_hash(&hash_v2).unwrap();
    assert!(
        record_v2.is_some(),
        "New hash must be recorded on overwrite"
    );
    let record_v2 = record_v2.unwrap();
    assert_eq!(record_v2.file_size, content_v2.len() as i64);

    let record_v1 = db.get_file_by_hash(&hash_v1).unwrap();
    assert!(
        record_v1.is_none(),
        "Old hash must no longer be present for overwritten file"
    );
}

#[tokio::test]
async fn test_webdav_delete_removes_from_database_and_disk() {
    let temp_dir = TempDir::new().unwrap();
    let config = Config::new_for_test(temp_dir.path());
    let db = Database::init(&config.db_path).unwrap();
    let app = create_app(config.clone(), db.clone()).await.unwrap();

    use sha2::{Digest, Sha256};
    let content = b"Temporary file to delete";
    let hash = hex::encode(Sha256::digest(content));

    // Upload
    let put_req = Request::builder()
        .method("PUT")
        .uri("/dav/Drive/temp.txt")
        .body(Body::from(content.to_vec()))
        .unwrap();
    let put_res = app.clone().oneshot(put_req).await.unwrap();
    assert!(put_res.status().is_success());
    assert!(db.get_file_by_hash(&hash).unwrap().is_some());

    // DELETE
    let del_req = Request::builder()
        .method("DELETE")
        .uri("/dav/Drive/temp.txt")
        .body(Body::empty())
        .unwrap();
    let del_res = app.clone().oneshot(del_req).await.unwrap();
    assert!(del_res.status().is_success());

    // Verify physical file is gone
    let file_path = config.storage_dir.join("Drive").join("temp.txt");
    assert!(!file_path.exists(), "Physical file must be deleted");

    // Verify DB record is gone
    assert!(
        db.get_file_by_hash(&hash).unwrap().is_none(),
        "Database record must be deleted"
    );

    // Verify HEAD /api/files/check/{hash} returns 404
    let head_req = Request::builder()
        .method("HEAD")
        .uri(format!("/api/files/check/{}", hash))
        .body(Body::empty())
        .unwrap();
    let head_res = app.oneshot(head_req).await.unwrap();
    assert_eq!(head_res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_nested_timeline_directory_creation_and_upload() {
    let temp_dir = TempDir::new().unwrap();
    let config = Config::new_for_test(temp_dir.path());
    let db = Database::init(&config.db_path).unwrap();
    let app = create_app(config.clone(), db.clone()).await.unwrap();

    use sha2::{Digest, Sha256};
    let photo_bytes = b"fake JPEG binary photo stream data";
    let photo_hash = hex::encode(Sha256::digest(photo_bytes));

    // MKCOL /dav/Timeline/2026
    let mkcol1 = Request::builder()
        .method("MKCOL")
        .uri("/dav/Timeline/2026")
        .body(Body::empty())
        .unwrap();
    let res_col1 = app.clone().oneshot(mkcol1).await.unwrap();
    assert!(res_col1.status().is_success());

    // MKCOL /dav/Timeline/2026/10
    let mkcol2 = Request::builder()
        .method("MKCOL")
        .uri("/dav/Timeline/2026/10")
        .body(Body::empty())
        .unwrap();
    let res_col2 = app.clone().oneshot(mkcol2).await.unwrap();
    assert!(res_col2.status().is_success());

    // PUT photo
    let put_req = Request::builder()
        .method("PUT")
        .uri("/dav/Timeline/2026/10/IMG_001.jpg")
        .body(Body::from(photo_bytes.to_vec()))
        .unwrap();
    let put_res = app.clone().oneshot(put_req).await.unwrap();
    assert!(put_res.status().is_success());

    // Verify file on disk
    let photo_path = config
        .storage_dir
        .join("Timeline")
        .join("2026")
        .join("10")
        .join("IMG_001.jpg");
    assert!(photo_path.exists());

    // Verify hash
    let record = db.get_file_by_hash(&photo_hash).unwrap();
    assert!(record.is_some());
    let record = record.unwrap();
    assert_eq!(record.file_name, "IMG_001.jpg");
}

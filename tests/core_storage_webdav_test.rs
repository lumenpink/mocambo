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
        put_res.status() == StatusCode::CREATED || put_res.status() == StatusCode::NO_CONTENT || put_res.status() == StatusCode::OK,
        "PUT failed with status: {}",
        put_res.status()
    );

    // 2. Verify physical POSIX persistence on disk
    let expected_file_path = config.storage_dir.join("Drive").join("hello.txt");
    assert!(expected_file_path.exists(), "File was not written to POSIX disk at {:?}", expected_file_path);
    let read_back = std::fs::read(&expected_file_path).unwrap();
    assert_eq!(read_back, file_content);

    // 3. Verify SQLite catalog record
    let record = db.get_file_by_hash(&expected_hash).unwrap();
    assert!(record.is_some(), "File record was not found in database by hash");
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

use axum::body::Body;
use axum::http::{Request, StatusCode};
use mocambo::app::create_app;
use mocambo::config::Config;
use mocambo::db::Database;
use tempfile::TempDir;
use tower::ServiceExt;

#[tokio::test]
async fn test_s3_put_get_and_delete_object() {
    let temp_dir = TempDir::new().unwrap();
    let config = Config::new_for_test(temp_dir.path());
    let db = Database::init(&config.db_path).unwrap();
    let app = create_app(config, db).await.unwrap();

    let content = b"S3 Minimal Storage Object Content";

    // 1. PutObject: PUT /s3/vault/notas/plano.txt
    let put_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/s3/vault/notas/plano.txt")
                .body(Body::from(content.to_vec()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(put_res.status(), StatusCode::OK);
    let etag = put_res
        .headers()
        .get("etag")
        .expect("ETag header presente no PutObject")
        .to_str()
        .unwrap()
        .to_string();
    assert!(!etag.is_empty());

    // 2. GetObject: GET /s3/vault/notas/plano.txt
    let get_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/s3/vault/notas/plano.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(get_res.status(), StatusCode::OK);
    let get_etag = get_res
        .headers()
        .get("etag")
        .expect("ETag presente no GetObject")
        .to_str()
        .unwrap();
    assert_eq!(etag, get_etag);

    let body = axum::body::to_bytes(get_res.into_body(), 1024 * 1024).await.unwrap();
    assert_eq!(body.as_ref(), content);

    // 3. ListObjectsV2: GET /s3/vault
    let list_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/s3/vault")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(list_res.status(), StatusCode::OK);
    let list_type = list_res
        .headers()
        .get("content-type")
        .expect("Content-Type em ListObjectsV2")
        .to_str()
        .unwrap();
    assert!(list_type.contains("xml"));

    let list_xml = axum::body::to_bytes(list_res.into_body(), 1024 * 1024).await.unwrap();
    let xml_str = String::from_utf8_lossy(&list_xml);
    assert!(xml_str.contains("<ListBucketResult"));
    assert!(xml_str.contains("<Name>vault</Name>"));
    assert!(xml_str.contains("<Key>notas/plano.txt</Key>"));

    // 4. DeleteObject: DELETE /s3/vault/notas/plano.txt
    let del_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/s3/vault/notas/plano.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(del_res.status(), StatusCode::NO_CONTENT);

    // 5. GetObject após deleção deve retornar 404 Not Found
    let get_del_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/s3/vault/notas/plano.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(get_del_res.status(), StatusCode::NOT_FOUND);

    // 6. Path Traversal bloqueado
    let traversal_res = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/s3/vault/../../etc/passwd")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(
        traversal_res.status() == StatusCode::BAD_REQUEST
            || traversal_res.status() == StatusCode::FORBIDDEN
            || traversal_res.status() == StatusCode::NOT_FOUND
    );
}

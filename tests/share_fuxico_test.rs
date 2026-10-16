use axum::extract::Path as AxumPath;
use axum::http::StatusCode;
use axum::routing::{delete as axum_delete, post};
use axum::{Json, Router};
use mocambo::app::create_app;
use mocambo::config::Config;
use mocambo::db::Database;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tempfile::tempdir;
use tokio::net::TcpListener;

struct MockFuxicoState {
    share_calls: AtomicUsize,
    delete_calls: AtomicUsize,
}

#[tokio::test]
async fn test_fuxico_share_lifecycle_and_internal_streaming() {
    // 1. Iniciar Mock Server do Fuxico
    let mock_state = Arc::new(MockFuxicoState {
        share_calls: AtomicUsize::new(0),
        delete_calls: AtomicUsize::new(0),
    });

    let mock_app_state = mock_state.clone();
    let mock_fuxico_router = Router::new()
        .route(
            "/api/mocambo/share",
            post({
                let state = mock_app_state.clone();
                move |Json(body): Json<Value>| {
                    let state = state.clone();
                    async move {
                        state.share_calls.fetch_add(1, Ordering::SeqCst);
                        assert_eq!(body["file_name"], "relatorio.pdf");
                        assert!(body["mocambo_token"]
                            .as_str()
                            .unwrap()
                            .starts_with("moca_tok_"));
                        assert!(body["mocambo_stream_url"]
                            .as_str()
                            .unwrap()
                            .contains("/api/internal/stream/"));

                        (
                            StatusCode::CREATED,
                            Json(json!({
                                "slug": "fux-moca-slug",
                                "url": "https://2lp.in/fux-moca-slug",
                                "x_token": "fux_delete_token_123"
                            })),
                        )
                    }
                }
            }),
        )
        .route(
            "/api/mocambo/share/{slug}",
            axum_delete({
                let state = mock_app_state.clone();
                move |AxumPath(slug): AxumPath<String>| {
                    let state = state.clone();
                    async move {
                        state.delete_calls.fetch_add(1, Ordering::SeqCst);
                        assert_eq!(slug, "fux-moca-slug");
                        (
                            StatusCode::OK,
                            Json(json!({
                                "success": true,
                                "slug": slug
                            })),
                        )
                    }
                }
            }),
        );

    let mock_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mock_addr = mock_listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(mock_listener, mock_fuxico_router)
            .await
            .unwrap();
    });

    // 2. Configurar ambiente Mocambo
    let temp = tempdir().unwrap();
    let mut config = Config::new_for_test(temp.path());
    config.fuxico_base_url = format!("http://{}", mock_addr);
    config.fuxico_api_token = "mestre-token-fuxico".to_string();
    config.mocambo_internal_url = "http://127.0.0.1:8080".to_string();

    let db = Database::init(&config.db_path).unwrap();

    // Criar arquivo real no disco POSIX
    let drive_dir = config.storage_dir.join("Drive");
    let file_path = drive_dir.join("relatorio.pdf");
    let file_content = b"PDF-1.4 Mock Stream Payload For Mocambo Share Test";
    tokio::fs::write(&file_path, file_content).await.unwrap();

    // Registrar arquivo no banco de dados local
    db.record_file(
        "default",
        "Drive/relatorio.pdf",
        "relatorio.pdf",
        file_content.len() as i64,
        "dummy_sha256",
        "application/pdf",
    )
    .unwrap();

    // 3. Iniciar App Mocambo com porta efêmera
    let app = create_app(config.clone(), db.clone()).await.unwrap();
    let moca_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let moca_addr = moca_listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(moca_listener, app).await.unwrap();
    });

    let client = reqwest::Client::new();
    let base_url = format!("http://{}", moca_addr);

    // 4. Teste: Criar Share via POST /api/shares
    let share_req = json!({
        "storage_path": "Drive/relatorio.pdf",
        "ttl": "24h"
    });

    let res = client
        .post(format!("{}/api/shares", base_url))
        .json(&share_req)
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), reqwest::StatusCode::CREATED);
    let share_data: Value = res.json().await.unwrap();
    assert_eq!(share_data["fuxico_slug"], "fux-moca-slug");
    assert_eq!(share_data["fuxico_url"], "https://2lp.in/fux-moca-slug");
    let token = share_data["token"].as_str().unwrap().to_string();
    assert!(token.starts_with("moca_tok_"));

    assert_eq!(mock_state.share_calls.load(Ordering::SeqCst), 1);

    // 5. Teste: Streaming Interno via GET /api/internal/stream/{token}
    let stream_res = client
        .get(format!("{}/api/internal/stream/{}", base_url, token))
        .send()
        .await
        .unwrap();

    assert_eq!(stream_res.status(), reqwest::StatusCode::OK);
    assert_eq!(
        stream_res
            .headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap(),
        "application/pdf"
    );
    let stream_bytes = stream_res.bytes().await.unwrap();
    assert_eq!(stream_bytes.as_ref(), file_content);

    // 6. Teste: Revogar Share via DELETE /api/shares/{token}
    let del_res = client
        .delete(format!("{}/api/shares/{}", base_url, token))
        .send()
        .await
        .unwrap();

    assert_eq!(del_res.status(), reqwest::StatusCode::OK);
    assert_eq!(mock_state.delete_calls.load(Ordering::SeqCst), 1);

    // 7. Teste: Streaming pós-revogação deve retornar 404 ou 410
    let stream_after_res = client
        .get(format!("{}/api/internal/stream/{}", base_url, token))
        .send()
        .await
        .unwrap();

    assert!(
        stream_after_res.status() == reqwest::StatusCode::NOT_FOUND
            || stream_after_res.status() == reqwest::StatusCode::GONE
    );
}

#[tokio::test]
async fn test_share_nonexistent_file_and_invalid_token() {
    let temp = tempdir().unwrap();
    let config = Config::new_for_test(temp.path());
    let db = Database::init(&config.db_path).unwrap();

    let app = create_app(config.clone(), db.clone()).await.unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let client = reqwest::Client::new();
    let base_url = format!("http://{}", addr);

    // 1. Share de arquivo inexistente retorna 404
    let res = client
        .post(format!("{}/api/shares", base_url))
        .json(&json!({
            "storage_path": "Drive/arquivo_fantasma.txt"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), reqwest::StatusCode::NOT_FOUND);

    // 2. Stream de token inexistente retorna 404
    let stream_res = client
        .get(format!(
            "{}/api/internal/stream/moca_tok_inexistente",
            base_url
        ))
        .send()
        .await
        .unwrap();

    assert_eq!(stream_res.status(), reqwest::StatusCode::NOT_FOUND);

    // 3. Delete de token inexistente retorna 404
    let del_res = client
        .delete(format!("{}/api/shares/moca_tok_inexistente", base_url))
        .send()
        .await
        .unwrap();

    assert_eq!(del_res.status(), reqwest::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_webdav_delete_revokes_active_share() {
    let mock_state = Arc::new(MockFuxicoState {
        share_calls: AtomicUsize::new(0),
        delete_calls: AtomicUsize::new(0),
    });

    let mock_app_state = mock_state.clone();
    let mock_fuxico_router = Router::new()
        .route(
            "/api/mocambo/share",
            post({
                let state = mock_app_state.clone();
                move |Json(_body): Json<Value>| {
                    let state = state.clone();
                    async move {
                        state.share_calls.fetch_add(1, Ordering::SeqCst);
                        (
                            StatusCode::CREATED,
                            Json(json!({
                                "slug": "slug-para-deletar",
                                "url": "https://2lp.in/slug-para-deletar",
                                "x_token": "fux_delete_token"
                            })),
                        )
                    }
                }
            }),
        )
        .route(
            "/api/mocambo/share/{slug}",
            axum_delete({
                let state = mock_app_state.clone();
                move |AxumPath(slug): AxumPath<String>| {
                    let state = state.clone();
                    async move {
                        state.delete_calls.fetch_add(1, Ordering::SeqCst);
                        (
                            StatusCode::OK,
                            Json(json!({
                                "success": true,
                                "slug": slug
                            })),
                        )
                    }
                }
            }),
        );

    let mock_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mock_addr = mock_listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(mock_listener, mock_fuxico_router)
            .await
            .unwrap();
    });

    let temp = tempdir().unwrap();
    let mut config = Config::new_for_test(temp.path());
    config.fuxico_base_url = format!("http://{}", mock_addr);
    let db = Database::init(&config.db_path).unwrap();

    let file_path = config.storage_dir.join("Drive").join("documento.txt");
    tokio::fs::write(&file_path, b"conteudo importante")
        .await
        .unwrap();
    db.record_file(
        "default",
        "Drive/documento.txt",
        "documento.txt",
        19,
        "dummy_hash",
        "text/plain",
    )
    .unwrap();

    let app = create_app(config.clone(), db.clone()).await.unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let client = reqwest::Client::new();
    let base_url = format!("http://{}", addr);

    // Cria a share
    let res = client
        .post(format!("{}/api/shares", base_url))
        .json(&json!({
            "storage_path": "Drive/documento.txt"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), reqwest::StatusCode::CREATED);
    let share_data: Value = res.json().await.unwrap();
    let token = share_data["token"].as_str().unwrap().to_string();

    // Deleta o arquivo via WebDAV DELETE /dav/Drive/documento.txt
    let dav_del_res = client
        .delete(format!("{}/dav/Drive/documento.txt", base_url))
        .send()
        .await
        .unwrap();

    assert!(dav_del_res.status().is_success());

    // Aguarda propagação assíncrona da revogação
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // A share no banco de dados deve estar marcada como revoked
    let share_record = db.get_share_by_token(&token).unwrap().unwrap();
    assert_eq!(share_record.status, "revoked");
    assert_eq!(mock_state.delete_calls.load(Ordering::SeqCst), 1);
}

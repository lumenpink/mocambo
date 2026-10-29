use axum::http::StatusCode;
use mocambo::app::create_app;
use mocambo::config::Config;
use mocambo::db::Database;
use serde_json::Value;
use std::time::Duration;
use tempfile::tempdir;
use tokio::net::TcpListener;

#[tokio::test]
async fn test_async_vision_embedding_and_semantic_image_search() {
    let temp = tempdir().unwrap();
    let config = Config::new_for_test(temp.path());
    let db = Database::init(&config.db_path).unwrap();

    let app = create_app(config.clone(), db.clone()).await.unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let base_url = format!("http://{}", addr);

    // 1. Busca inicial com repositório vazio retorna 0 resultados
    let empty_search_res = client
        .get(format!("{}/api/search?q=praia", base_url))
        .send()
        .await
        .unwrap();

    assert_eq!(empty_search_res.status(), StatusCode::OK);
    let empty_data: Value = empty_search_res.json().await.unwrap();
    assert_eq!(empty_data["total"], 0);

    // 2. Upload de imagem simulada via WebDAV PUT
    let fake_image_bytes = vec![
        0xFF, 0xD8, 0xFF, 0xFE, 0x00, 0x11, b'p', b'r', b'a', b'i', b'a', b' ', b's', b'o', b'l',
        b' ', b'm', b'a', b'r', 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46, 0x00, 0x01, 0x01,
        0x01, 0x00, 0x48, 0x00, 0x48, 0x00, 0x00, 0xFF, 0xDB, 0x00, 0x43, 0x00, 0x08, 0x06, 0x06,
        0x07, 0x06, 0x05, 0x08, 0x07, 0x07, 0x07, 0x09, 0x09, 0x08, 0x0A, 0x0C, 0x14, 0x0D, 0x0C,
        0x0B, 0x0B, 0x0C, 0x19, 0x12, 0x13, 0x0F, 0x14, 0x1D, 0x1A, 0x1F, 0x1E, 0x1D, 0x1A, 0x1C,
        0x1C, 0x20, 0x24, 0x2E, 0x27, 0x20, 0x22, 0x2C, 0x23, 0x1C, 0x1C, 0x28, 0x37, 0x29, 0x2C,
        0x30, 0x31, 0x34, 0x34, 0x34, 0x1F, 0x27, 0x39, 0x3D, 0x38, 0x32, 0x3C, 0x2E, 0x33, 0x34,
        0x32, 0xFF, 0xC0, 0x00, 0x0B, 0x08, 0x00, 0x01, 0x00, 0x01, 0x01, 0x01, 0x11, 0x00, 0xFF,
        0xC4, 0x00, 0x1F, 0x00, 0x00, 0x01, 0x05, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09,
        0x0A, 0x0B, 0xFF, 0xDA, 0x00, 0x08, 0x01, 0x01, 0x00, 0x00, 0x3F, 0x00, 0x7F, 0x00, 0xFF,
        0xD9,
    ];

    let put_res = client
        .put(format!("{}/dav/Timeline/IMG_20261016_0042.jpg", base_url))
        .body(fake_image_bytes)
        .send()
        .await
        .unwrap();

    assert!(put_res.status().is_success());

    // 3. Aguardar indexação assíncrona do vetor visual em background
    let mut found = false;
    for _ in 0..30 {
        tokio::time::sleep(Duration::from_millis(50)).await;
        let search_res = client
            .get(format!("{}/api/search?q=praia", base_url))
            .send()
            .await
            .unwrap();

        if search_res.status() == StatusCode::OK {
            let data: Value = search_res.json().await.unwrap();
            if data["total"].as_u64().unwrap_or(0) > 0 {
                let first = &data["results"][0];
                assert_eq!(first["file_name"], "IMG_20261016_0042.jpg");
                assert_eq!(first["storage_path"], "Timeline/IMG_20261016_0042.jpg");
                assert_eq!(first["mime_type"], "image/jpeg");
                let match_type = first["match_type"].as_str().unwrap_or("");
                assert!(
                    match_type == "semantic_image" || match_type == "hybrid",
                    "Tipo de correspondência esperado: semantic_image ou hybrid, obtido: {}",
                    match_type
                );
                found = true;
                break;
            }
        }
    }

    assert!(
        found,
        "A imagem não foi indexada ou encontrada na busca semântica"
    );

    // 4. Excluir a imagem via WebDAV DELETE e verificar remoção da busca
    let del_res = client
        .delete(format!("{}/dav/Timeline/IMG_20261016_0042.jpg", base_url))
        .send()
        .await
        .unwrap();
    assert!(del_res.status().is_success());

    tokio::time::sleep(Duration::from_millis(50)).await;

    let search_after_del = client
        .get(format!("{}/api/search?q=praia", base_url))
        .send()
        .await
        .unwrap();
    let after_del_data: Value = search_after_del.json().await.unwrap();
    assert_eq!(after_del_data["total"], 0);
}

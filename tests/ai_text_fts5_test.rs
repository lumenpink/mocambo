use axum::http::StatusCode;
use mocambo::app::create_app;
use mocambo::config::Config;
use mocambo::db::Database;
use serde_json::Value;
use std::time::Duration;
use tempfile::tempdir;
use tokio::net::TcpListener;

#[tokio::test]
async fn test_async_text_extraction_and_fts5_search() {
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

    // 1. Busca inicial com repositório vazio retorna 0 resultados
    let empty_search_res = client
        .get(format!("{}/api/search?q=bizantino", base_url))
        .send()
        .await
        .unwrap();

    assert_eq!(empty_search_res.status(), StatusCode::OK);
    let empty_data: Value = empty_search_res.json().await.unwrap();
    assert_eq!(empty_data["total"], 0);

    // 2. Upload de arquivo de texto via WebDAV PUT
    let file_content =
        "Este documento descreve o algoritmo de consenso bizantino avançado para nós distribuídos.";
    let put_res = client
        .put(format!("{}/dav/Drive/algoritmo.txt", base_url))
        .body(file_content)
        .send()
        .await
        .unwrap();

    assert!(put_res.status().is_success());

    // 3. Aguardar indexação assíncrona pelo worker em segundo plano
    let mut indexed = false;
    for _ in 0..30 {
        tokio::time::sleep(Duration::from_millis(50)).await;
        let search_res = client
            .get(format!("{}/api/search?q=bizantino", base_url))
            .send()
            .await
            .unwrap();

        if search_res.status() == StatusCode::OK {
            let data: Value = search_res.json().await.unwrap();
            if data["total"].as_u64().unwrap_or(0) > 0 {
                let first = &data["results"][0];
                assert_eq!(first["file_name"], "algoritmo.txt");
                assert_eq!(first["storage_path"], "Drive/algoritmo.txt");
                let snippet = first["snippet"].as_str().unwrap();
                assert!(snippet.contains("bizantino"));
                indexed = true;
                break;
            }
        }
    }

    assert!(
        indexed,
        "O arquivo não foi indexado a tempo pelo worker FTS5"
    );

    // 4. Busca por outro termo presente no texto
    let search_consenso = client
        .get(format!("{}/api/search?q=consenso", base_url))
        .send()
        .await
        .unwrap();
    let consenso_data: Value = search_consenso.json().await.unwrap();
    assert_eq!(consenso_data["total"], 1);

    // 5. Busca por termo inexistente retorna 0
    let search_inexistente = client
        .get(format!(
            "{}/api/search?q=termoinexistentenodocumento",
            base_url
        ))
        .send()
        .await
        .unwrap();
    let inexistente_data: Value = search_inexistente.json().await.unwrap();
    assert_eq!(inexistente_data["total"], 0);

    // 6. Excluir o arquivo via WebDAV DELETE e verificar remoção do índice FTS5
    let del_res = client
        .delete(format!("{}/dav/Drive/algoritmo.txt", base_url))
        .send()
        .await
        .unwrap();
    assert!(del_res.status().is_success());

    tokio::time::sleep(Duration::from_millis(50)).await;

    let search_after_del = client
        .get(format!("{}/api/search?q=bizantino", base_url))
        .send()
        .await
        .unwrap();
    let after_del_data: Value = search_after_del.json().await.unwrap();
    assert_eq!(after_del_data["total"], 0);
}

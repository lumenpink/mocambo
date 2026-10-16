use crate::api::{router as api_router, AppState};
use crate::config::Config;
use crate::db::Database;
use crate::webdav::InterceptingFs;
use axum::body::Body;
use axum::extract::Request;
use axum::Router;
use dav_server::DavHandler;
use std::sync::Arc;

pub async fn create_app(
    config: Config,
    db: Database,
) -> Result<Router, Box<dyn std::error::Error>> {
    let user_id = "default".to_string();
    let fuxico = Arc::new(crate::fuxico::FuxicoClient::new(
        config.fuxico_base_url.clone(),
        config.fuxico_api_token.clone(),
    ));

    let (indexer_tx, indexer_rx) = tokio::sync::mpsc::channel(512);
    let indexer_handle = crate::indexer::IndexerHandle::new(indexer_tx);
    let vision = crate::vision::create_default_embedder();
    crate::indexer::start_indexer(
        indexer_rx,
        db.clone(),
        config.storage_dir.clone(),
        vision.clone(),
    );

    let fs = InterceptingFs::new_with_fuxico_and_indexer(
        config.storage_dir.clone(),
        db.clone(),
        user_id,
        Some(fuxico.clone()),
        Some(indexer_handle),
    );
    let dav_handler = Arc::new(
        DavHandler::builder()
            .strip_prefix("/dav")
            .filesystem(fs)
            .build_handler(),
    );

    let state = AppState {
        db,
        config: config.clone(),
        fuxico,
        vision,
    };
    let app = api_router(state).fallback(move |req: Request| {
        let handler = dav_handler.clone();
        async move {
            let res = handler.handle(req).await;
            res.map(Body::new)
        }
    });

    Ok(app)
}

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

    let fs = InterceptingFs::new_with_fuxico(
        config.storage_dir.clone(),
        db.clone(),
        user_id,
        Some(fuxico.clone()),
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

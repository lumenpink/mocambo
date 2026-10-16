use mocambo::app::create_app;
use mocambo::config::Config;
use mocambo::db::Database;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let config = Config::from_env();
    info!("Iniciando Mocambo Storage...");
    info!("Storage dir: {:?}", config.storage_dir);
    info!("DB path: {:?}", config.db_path);

    let db = Database::init(&config.db_path)?;
    let app = create_app(config.clone(), db).await?;

    let listener = tokio::net::TcpListener::bind(&config.bind_addr).await?;
    info!("Mocambo rodando em http://{}", listener.local_addr()?);

    axum::serve(listener, app).await?;
    Ok(())
}

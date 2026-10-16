use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Config {
    pub storage_dir: PathBuf,
    pub db_path: PathBuf,
    pub bind_addr: String,
}

impl Config {
    pub fn new_for_test(temp_dir: &Path) -> Self {
        let storage_dir = temp_dir.join("storage");
        let db_path = temp_dir.join("mocambo.db");
        std::fs::create_dir_all(&storage_dir).expect("Failed to create test storage dir");
        std::fs::create_dir_all(storage_dir.join("Drive")).expect("Failed to create Drive dir");
        std::fs::create_dir_all(storage_dir.join("Timeline"))
            .expect("Failed to create Timeline dir");

        Self {
            storage_dir,
            db_path,
            bind_addr: "127.0.0.1:0".to_string(),
        }
    }

    pub fn from_env() -> Self {
        let storage_dir = std::env::var("MOCAMBO_DATA_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("./data/storage"));
        let db_path = std::env::var("MOCAMBO_DB_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("./data/mocambo.db"));
        let bind_addr =
            std::env::var("MOCAMBO_BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_string());

        if let Some(parent) = db_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::create_dir_all(&storage_dir);
        let _ = std::fs::create_dir_all(storage_dir.join("Drive"));
        let _ = std::fs::create_dir_all(storage_dir.join("Timeline"));

        Self {
            storage_dir,
            db_path,
            bind_addr,
        }
    }
}

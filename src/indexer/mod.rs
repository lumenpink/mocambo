use crate::db::Database;
use std::path::{Path, PathBuf};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

#[derive(Debug, Clone)]
pub struct IndexTask {
    pub storage_path: String,
    pub rel_path: PathBuf,
}

#[derive(Clone)]
pub struct IndexerHandle {
    sender: mpsc::Sender<IndexTask>,
}

impl IndexerHandle {
    pub fn new(sender: mpsc::Sender<IndexTask>) -> Self {
        Self { sender }
    }

    pub fn enqueue(&self, task: IndexTask) {
        if let Err(e) = self.sender.try_send(task) {
            warn!("Fila de indexação cheia ou worker indisponível: {}", e);
        }
    }
}

pub fn start_indexer(
    mut receiver: mpsc::Receiver<IndexTask>,
    db: Database,
    storage_dir: PathBuf,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        info!("Worker de indexação FTS5 iniciado com sucesso");
        while let Some(task) = receiver.recv().await {
            let db_clone = db.clone();
            let storage_dir_clone = storage_dir.clone();
            if let Err(e) = process_index_task(&db_clone, &storage_dir_clone, &task).await {
                warn!(
                    "Falha ao processar indexação de {}: {}",
                    task.storage_path, e
                );
            }
        }
        info!("Worker de indexação FTS5 finalizado");
    })
}

async fn process_index_task(
    db: &Database,
    storage_dir: &Path,
    task: &IndexTask,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let full_path = storage_dir.join(&task.rel_path);
    if !full_path.exists() || !full_path.is_file() {
        debug!(
            "Arquivo ignorado ou não encontrado no disco para indexação: {:?}",
            full_path
        );
        return Ok(());
    }

    let file_rec = match db.get_file_by_storage_path(&task.storage_path)? {
        Some(f) => f,
        None => {
            debug!(
                "Registro de arquivo não encontrado no banco para {}",
                task.storage_path
            );
            return Ok(());
        }
    };

    let text_content = extract_text_from_file(&full_path, &file_rec.mime_type).await;
    db.index_file_content(
        file_rec.id,
        &file_rec.storage_path,
        &file_rec.file_name,
        &text_content,
    )?;

    debug!(
        "Arquivo {} indexado no FTS5 com sucesso ({} caracteres)",
        file_rec.storage_path,
        text_content.len()
    );

    Ok(())
}

async fn extract_text_from_file(path: &Path, mime_type: &str) -> String {
    const MAX_READ_BYTES: usize = 10 * 1024 * 1024; // 10 MB

    let is_text = mime_type.starts_with("text/")
        || mime_type == "application/json"
        || mime_type == "application/xml"
        || mime_type == "application/javascript"
        || mime_type == "application/x-yaml"
        || mime_type == "application/x-sh"
        || path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|ext| {
                matches!(
                    ext.to_lowercase().as_str(),
                    "txt"
                        | "md"
                        | "markdown"
                        | "json"
                        | "csv"
                        | "log"
                        | "rs"
                        | "py"
                        | "js"
                        | "ts"
                        | "html"
                        | "css"
                        | "yml"
                        | "yaml"
                        | "toml"
                        | "sh"
                        | "xml"
                )
            });

    if is_text {
        if let Ok(bytes) = tokio::fs::read(path).await {
            let slice = if bytes.len() > MAX_READ_BYTES {
                &bytes[..MAX_READ_BYTES]
            } else {
                &bytes[..]
            };
            return String::from_utf8_lossy(slice).to_string();
        }
    }

    let is_pdf = mime_type == "application/pdf"
        || path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("pdf"));

    if is_pdf {
        if let Ok(bytes) = tokio::fs::read(path).await {
            return extract_text_from_pdf(&bytes);
        }
    }

    String::new()
}

fn extract_text_from_pdf(bytes: &[u8]) -> String {
    let mut text = String::new();
    let mut in_parentheses = false;
    let mut current_word = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'\\' && in_parentheses && i + 1 < bytes.len() {
            current_word.push(bytes[i + 1]);
            i += 2;
            continue;
        }
        if b == b'(' && !in_parentheses {
            in_parentheses = true;
            current_word.clear();
        } else if b == b')' && in_parentheses {
            in_parentheses = false;
            if !current_word.is_empty() {
                let word = String::from_utf8_lossy(&current_word);
                text.push_str(&word);
                text.push(' ');
                current_word.clear();
            }
        } else if in_parentheses {
            current_word.push(b);
        }
        i += 1;
    }
    text
}

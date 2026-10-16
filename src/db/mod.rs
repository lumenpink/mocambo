use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileRecord {
    pub id: i64,
    pub user_id: String,
    pub storage_path: String,
    pub file_name: String,
    pub file_size: i64,
    pub sha256: String,
    pub mime_type: String,
    pub created_at: i64,
    pub mtime: i64,
    pub analysis_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareRecord {
    pub id: String,
    pub token: String,
    pub user_id: String,
    pub storage_path: String,
    pub fuxico_slug: String,
    pub fuxico_url: String,
    pub created_at: i64,
    pub expires_at: Option<i64>,
    pub status: String,
}

#[derive(Clone)]
pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    pub fn init(path: &Path) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(path)?;

        // Modo WAL e pragmas recomendados
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys = ON;
             
             CREATE TABLE IF NOT EXISTS files (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 user_id TEXT NOT NULL,
                 storage_path TEXT NOT NULL UNIQUE,
                 file_name TEXT NOT NULL,
                 file_size INTEGER NOT NULL,
                 sha256 TEXT NOT NULL,
                 mime_type TEXT NOT NULL,
                 created_at INTEGER NOT NULL,
                 mtime INTEGER NOT NULL,
                 analysis_status TEXT NOT NULL DEFAULT 'pending'
             );
             
             CREATE INDEX IF NOT EXISTS idx_files_sha256 ON files(sha256);
             CREATE INDEX IF NOT EXISTS idx_files_path ON files(storage_path);

             CREATE TABLE IF NOT EXISTS shares (
                 id TEXT PRIMARY KEY,
                 token TEXT NOT NULL UNIQUE,
                 user_id TEXT NOT NULL,
                 storage_path TEXT NOT NULL,
                 fuxico_slug TEXT NOT NULL,
                 fuxico_url TEXT NOT NULL,
                 created_at INTEGER NOT NULL,
                 expires_at INTEGER NULL,
                 status TEXT NOT NULL DEFAULT 'active'
             );

             CREATE INDEX IF NOT EXISTS idx_shares_token ON shares(token);
             CREATE INDEX IF NOT EXISTS idx_shares_storage_path ON shares(storage_path);
             CREATE INDEX IF NOT EXISTS idx_shares_fuxico_slug ON shares(fuxico_slug);",
        )?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn record_file(
        &self,
        user_id: &str,
        storage_path: &str,
        file_name: &str,
        file_size: i64,
        sha256: &str,
        mime_type: &str,
    ) -> Result<(), rusqlite::Error> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;

        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO files (user_id, storage_path, file_name, file_size, sha256, mime_type, created_at, mtime, analysis_status)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7, 'pending')
             ON CONFLICT(storage_path) DO UPDATE SET
                 file_size = excluded.file_size,
                 sha256 = excluded.sha256,
                 mime_type = excluded.mime_type,
                 mtime = excluded.mtime",
            params![user_id, storage_path, file_name, file_size, sha256, mime_type, now],
        )?;

        Ok(())
    }

    pub fn get_file_by_hash(&self, hash: &str) -> Result<Option<FileRecord>, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, user_id, storage_path, file_name, file_size, sha256, mime_type, created_at, mtime, analysis_status
             FROM files WHERE sha256 = ?1 LIMIT 1",
        )?;

        let mut rows = stmt.query(params![hash])?;
        if let Some(row) = rows.next()? {
            Ok(Some(FileRecord {
                id: row.get(0)?,
                user_id: row.get(1)?,
                storage_path: row.get(2)?,
                file_name: row.get(3)?,
                file_size: row.get(4)?,
                sha256: row.get(5)?,
                mime_type: row.get(6)?,
                created_at: row.get(7)?,
                mtime: row.get(8)?,
                analysis_status: row.get(9)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn rename_file(
        &self,
        from_path: &str,
        to_path: &str,
        new_file_name: &str,
    ) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE files SET storage_path = ?1, file_name = ?2 WHERE storage_path = ?3",
            params![to_path, new_file_name, from_path],
        )?;
        Ok(())
    }

    pub fn remove_file_by_path(&self, storage_path: &str) -> Result<(), rusqlite::Error> {
        let clean = storage_path.trim_start_matches('/');
        let with_slash = format!("/{}", clean);
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM files WHERE storage_path = ?1 OR storage_path = ?2",
            params![clean, with_slash],
        )?;
        Ok(())
    }

    pub fn get_file_by_storage_path(
        &self,
        storage_path: &str,
    ) -> Result<Option<FileRecord>, rusqlite::Error> {
        let clean = storage_path.trim_start_matches('/');
        let with_slash = format!("/{}", clean);
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, user_id, storage_path, file_name, file_size, sha256, mime_type, created_at, mtime, analysis_status
             FROM files WHERE storage_path = ?1 OR storage_path = ?2 LIMIT 1",
        )?;

        let mut rows = stmt.query(params![clean, with_slash])?;
        if let Some(row) = rows.next()? {
            Ok(Some(FileRecord {
                id: row.get(0)?,
                user_id: row.get(1)?,
                storage_path: row.get(2)?,
                file_name: row.get(3)?,
                file_size: row.get(4)?,
                sha256: row.get(5)?,
                mime_type: row.get(6)?,
                created_at: row.get(7)?,
                mtime: row.get(8)?,
                analysis_status: row.get(9)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn create_share(&self, share: &ShareRecord) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO shares (id, token, user_id, storage_path, fuxico_slug, fuxico_url, created_at, expires_at, status)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                share.id,
                share.token,
                share.user_id,
                share.storage_path,
                share.fuxico_slug,
                share.fuxico_url,
                share.created_at,
                share.expires_at,
                share.status
            ],
        )?;
        Ok(())
    }

    pub fn get_share_by_token(&self, token: &str) -> Result<Option<ShareRecord>, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, token, user_id, storage_path, fuxico_slug, fuxico_url, created_at, expires_at, status
             FROM shares WHERE token = ?1 LIMIT 1",
        )?;

        let mut rows = stmt.query(params![token])?;
        if let Some(row) = rows.next()? {
            Ok(Some(ShareRecord {
                id: row.get(0)?,
                token: row.get(1)?,
                user_id: row.get(2)?,
                storage_path: row.get(3)?,
                fuxico_slug: row.get(4)?,
                fuxico_url: row.get(5)?,
                created_at: row.get(6)?,
                expires_at: row.get(7)?,
                status: row.get(8)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn revoke_share(&self, token: &str) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE shares SET status = 'revoked' WHERE token = ?1",
            params![token],
        )?;
        Ok(())
    }

    pub fn get_shares_by_storage_path(
        &self,
        storage_path: &str,
    ) -> Result<Vec<ShareRecord>, rusqlite::Error> {
        let clean = storage_path.trim_start_matches('/');
        let with_slash = format!("/{}", clean);
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, token, user_id, storage_path, fuxico_slug, fuxico_url, created_at, expires_at, status
             FROM shares WHERE (storage_path = ?1 OR storage_path = ?2) AND status = 'active'",
        )?;

        let rows = stmt.query_map(params![clean, with_slash], |row| {
            Ok(ShareRecord {
                id: row.get(0)?,
                token: row.get(1)?,
                user_id: row.get(2)?,
                storage_path: row.get(3)?,
                fuxico_slug: row.get(4)?,
                fuxico_url: row.get(5)?,
                created_at: row.get(6)?,
                expires_at: row.get(7)?,
                status: row.get(8)?,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }
}

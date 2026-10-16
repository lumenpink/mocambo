use crate::db::Database;
use dav_server::davpath::DavPath;
use dav_server::fs::{
    DavDirEntry, DavFile, DavFileSystem, DavMetaData, DavProp, FsFuture, FsStream, OpenOptions,
    ReadDirMeta,
};
use dav_server::localfs::LocalFs;
use futures_util::future::FutureExt;
use http::StatusCode;
use sha2::{Digest, Sha256};
use std::fmt::Debug;
use std::fs::File;
use std::io::{BufReader, Read, SeekFrom};
use std::path::PathBuf;
use std::pin::Pin;

use std::sync::Arc;

#[derive(Clone)]
pub struct InterceptingFs {
    inner: Arc<LocalFs>,
    db: Database,
    root: PathBuf,
    user_id: String,
    fuxico: Option<Arc<crate::fuxico::FuxicoClient>>,
    indexer: Option<crate::indexer::IndexerHandle>,
}

impl InterceptingFs {
    pub fn new(root: PathBuf, db: Database, user_id: String) -> Box<Self> {
        Self::new_with_fuxico_and_indexer(root, db, user_id, None, None)
    }

    pub fn new_with_fuxico(
        root: PathBuf,
        db: Database,
        user_id: String,
        fuxico: Option<Arc<crate::fuxico::FuxicoClient>>,
    ) -> Box<Self> {
        Self::new_with_fuxico_and_indexer(root, db, user_id, fuxico, None)
    }

    pub fn new_with_fuxico_and_indexer(
        root: PathBuf,
        db: Database,
        user_id: String,
        fuxico: Option<Arc<crate::fuxico::FuxicoClient>>,
        indexer: Option<crate::indexer::IndexerHandle>,
    ) -> Box<Self> {
        let local_box = LocalFs::new(&root, false, false, false);
        let inner = Arc::new(*local_box);
        Box::new(Self {
            inner,
            db,
            root,
            user_id,
            fuxico,
            indexer,
        })
    }
}

impl DavFileSystem for InterceptingFs {
    fn open<'a>(
        &'a self,
        path: &'a DavPath,
        options: OpenOptions,
    ) -> FsFuture<'a, Box<dyn DavFile>> {
        async move {
            let is_write = options.write || options.create || options.append;
            let file = self.inner.open(path, options).await?;
            if is_write {
                Ok(Box::new(InterceptingFile {
                    inner: file,
                    db: self.db.clone(),
                    root: self.root.clone(),
                    rel_path: path.as_rel_ospath().to_path_buf(),
                    dav_path_str: path.as_url_string().to_string(),
                    user_id: self.user_id.clone(),
                    indexer: self.indexer.clone(),
                }) as Box<dyn DavFile>)
            } else {
                Ok(file)
            }
        }
        .boxed()
    }

    fn read_dir<'a>(
        &'a self,
        path: &'a DavPath,
        meta: ReadDirMeta,
    ) -> FsFuture<'a, FsStream<Box<dyn DavDirEntry>>> {
        self.inner.read_dir(path, meta)
    }

    fn metadata<'a>(&'a self, path: &'a DavPath) -> FsFuture<'a, Box<dyn DavMetaData>> {
        self.inner.metadata(path)
    }

    fn symlink_metadata<'a>(&'a self, path: &'a DavPath) -> FsFuture<'a, Box<dyn DavMetaData>> {
        self.inner.symlink_metadata(path)
    }

    fn create_dir<'a>(&'a self, path: &'a DavPath) -> FsFuture<'a, ()> {
        self.inner.create_dir(path)
    }

    fn remove_dir<'a>(&'a self, path: &'a DavPath) -> FsFuture<'a, ()> {
        self.inner.remove_dir(path)
    }

    fn remove_file<'a>(&'a self, path: &'a DavPath) -> FsFuture<'a, ()> {
        async move {
            self.inner.remove_file(path).await?;
            let path_str = path.as_url_string();
            let _ = self.db.remove_file_by_path(&path_str);
            let _ = self.db.remove_fts_by_path(&path_str);
            let _ = self.db.remove_vec_by_path(&path_str);
            if let Ok(shares) = self.db.get_shares_by_storage_path(&path_str) {
                for s in shares {
                    let _ = self.db.revoke_share(&s.token);
                    if let Some(fuxico) = &self.fuxico {
                        let fuxico = fuxico.clone();
                        tokio::spawn(async move {
                            let _ = fuxico.revoke_share(&s.fuxico_slug).await;
                        });
                    }
                }
            }
            Ok(())
        }
        .boxed()
    }

    fn rename<'a>(&'a self, from: &'a DavPath, to: &'a DavPath) -> FsFuture<'a, ()> {
        async move {
            self.inner.rename(from, to).await?;
            let new_name = to
                .as_rel_ospath()
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            let _ = self
                .db
                .rename_file(&from.as_url_string(), &to.as_url_string(), &new_name);
            let _ = self.db.remove_fts_by_path(&from.as_url_string());
            let _ = self.db.remove_vec_by_path(&from.as_url_string());
            if let Some(indexer) = &self.indexer {
                indexer.enqueue(crate::indexer::IndexTask {
                    storage_path: to.as_url_string(),
                    rel_path: to.as_rel_ospath().to_path_buf(),
                });
            }
            if let Ok(shares) = self.db.get_shares_by_storage_path(&from.as_url_string()) {
                for s in shares {
                    let _ = self.db.revoke_share(&s.token);
                    if let Some(fuxico) = &self.fuxico {
                        let fuxico = fuxico.clone();
                        tokio::spawn(async move {
                            let _ = fuxico.revoke_share(&s.fuxico_slug).await;
                        });
                    }
                }
            }
            Ok(())
        }
        .boxed()
    }

    fn copy<'a>(&'a self, from: &'a DavPath, to: &'a DavPath) -> FsFuture<'a, ()> {
        self.inner.copy(from, to)
    }

    fn have_props<'a>(
        &'a self,
        path: &'a DavPath,
    ) -> Pin<Box<dyn std::future::Future<Output = bool> + Send + 'a>> {
        self.inner.have_props(path)
    }

    fn patch_props<'a>(
        &'a self,
        path: &'a DavPath,
        patch: Vec<(bool, DavProp)>,
    ) -> FsFuture<'a, Vec<(StatusCode, DavProp)>> {
        self.inner.patch_props(path, patch)
    }

    fn get_props<'a>(&'a self, path: &'a DavPath, do_content: bool) -> FsFuture<'a, Vec<DavProp>> {
        self.inner.get_props(path, do_content)
    }

    fn get_prop<'a>(&'a self, path: &'a DavPath, prop: DavProp) -> FsFuture<'a, Vec<u8>> {
        self.inner.get_prop(path, prop)
    }

    fn get_quota(&'_ self) -> FsFuture<'_, (u64, Option<u64>)> {
        self.inner.get_quota()
    }
}

pub struct InterceptingFile {
    inner: Box<dyn DavFile>,
    db: Database,
    root: PathBuf,
    rel_path: PathBuf,
    dav_path_str: String,
    user_id: String,
    indexer: Option<crate::indexer::IndexerHandle>,
}

impl Debug for InterceptingFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InterceptingFile")
            .field("rel_path", &self.rel_path)
            .field("dav_path_str", &self.dav_path_str)
            .finish()
    }
}

impl DavFile for InterceptingFile {
    fn metadata(&'_ mut self) -> FsFuture<'_, Box<dyn DavMetaData>> {
        self.inner.metadata()
    }

    fn write_buf(&'_ mut self, buf: Box<dyn bytes::Buf + Send>) -> FsFuture<'_, ()> {
        self.inner.write_buf(buf)
    }

    fn write_bytes(&'_ mut self, buf: bytes::Bytes) -> FsFuture<'_, ()> {
        self.inner.write_bytes(buf)
    }

    fn read_bytes(&'_ mut self, count: usize) -> FsFuture<'_, bytes::Bytes> {
        self.inner.read_bytes(count)
    }

    fn seek(&'_ mut self, pos: SeekFrom) -> FsFuture<'_, u64> {
        self.inner.seek(pos)
    }

    fn flush(&'_ mut self) -> FsFuture<'_, ()> {
        async move {
            self.inner.flush().await?;
            let full_path = self.root.join(&self.rel_path);
            if full_path.is_file() {
                if let Ok(file) = File::open(&full_path) {
                    let mut reader = BufReader::new(file);
                    let mut hasher = Sha256::new();
                    let mut buffer = [0u8; 8192];
                    let mut total_size = 0i64;
                    while let Ok(n) = reader.read(&mut buffer) {
                        if n == 0 {
                            break;
                        }
                        hasher.update(&buffer[..n]);
                        total_size += n as i64;
                    }
                    let hash = hex::encode(hasher.finalize());
                    let file_name = self
                        .rel_path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| "unknown".to_string());
                    let mime = mime_guess::from_path(&full_path)
                        .first_or_octet_stream()
                        .to_string();

                    let _ = self.db.record_file(
                        &self.user_id,
                        &self.dav_path_str,
                        &file_name,
                        total_size,
                        &hash,
                        &mime,
                    );

                    if let Some(indexer) = &self.indexer {
                        indexer.enqueue(crate::indexer::IndexTask {
                            storage_path: self.dav_path_str.clone(),
                            rel_path: self.rel_path.clone(),
                        });
                    }
                }
            }
            Ok(())
        }
        .boxed()
    }
}

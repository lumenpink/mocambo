use axum::body::Body;
use axum::extract::{Query, State};
use axum::http::header::{CONTENT_LENGTH, CONTENT_TYPE};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Json, Response};
use serde::{Deserialize, Serialize};
use std::fs;
use std::time::UNIX_EPOCH;
use tokio_util::io::ReaderStream;

use crate::api::AppState;

#[derive(Debug, Deserialize)]
pub struct ListFilesQuery {
    pub path: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FileItem {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: u64,
    pub mime_type: String,
}

pub async fn web_ui_handler() -> Response {
    let html = WEB_UI_HTML;
    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static("text/html; charset=utf-8"),
    );
    headers.insert(
        "Content-Security-Policy",
        HeaderValue::from_static(
            "default-src 'self' 'unsafe-inline' 'unsafe-eval' data: blob:;",
        ),
    );
    (StatusCode::OK, headers, html).into_response()
}

pub async fn list_files_handler(
    State(state): State<AppState>,
    Query(query): Query<ListFilesQuery>,
) -> Response {
    let req_path = query.path.unwrap_or_else(|| "/".to_string());

    // Prevenção contra Directory Traversal
    if req_path.contains("..") {
        return (StatusCode::BAD_REQUEST, "Caminho inválido").into_response();
    }

    let clean_rel = req_path.trim_start_matches('/');
    let target_dir = state.config.storage_dir.join(clean_rel);

    // Validar se o target_dir está contido em storage_dir
    if let (Ok(canon_target), Ok(canon_root)) =
        (target_dir.canonicalize(), state.config.storage_dir.canonicalize())
    {
        if !canon_target.starts_with(&canon_root) {
            return (StatusCode::FORBIDDEN, "Acesso proibido").into_response();
        }
    } else if !target_dir.exists() {
        return (StatusCode::NOT_FOUND, "Diretório não encontrado").into_response();
    }

    let mut items = Vec::new();
    let entries = match fs::read_dir(&target_dir) {
        Ok(e) => e,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Erro ao ler diretório").into_response(),
    };

    for entry in entries.flatten() {
        let file_name = entry.file_name().to_string_lossy().to_string();
        // Ocultar arquivos internos ou ocultos
        if file_name.starts_with('.') {
            continue;
        }

        let file_type = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };

        let is_dir = file_type.is_dir();
        let metadata = entry.metadata().ok();
        let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
        let modified = metadata
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let item_rel = if clean_rel.is_empty() {
            format!("/{}", file_name)
        } else {
            format!("/{}/{}", clean_rel.trim_end_matches('/'), file_name)
        };

        let mime_type = if is_dir {
            "inode/directory".to_string()
        } else {
            mime_guess::from_path(&entry.path())
                .first_or_octet_stream()
                .to_string()
        };

        items.push(FileItem {
            name: file_name,
            path: item_rel,
            is_dir,
            size,
            modified,
            mime_type,
        });
    }

    // Ordenar: pastas primeiro, depois alfabético
    items.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    (StatusCode::OK, Json(items)).into_response()
}

pub async fn raw_file_handler(
    State(state): State<AppState>,
    axum::extract::Path(path): axum::extract::Path<String>,
) -> Response {
    if path.contains("..") {
        return (StatusCode::BAD_REQUEST, "Caminho inválido").into_response();
    }

    let clean_rel = path.trim_start_matches('/');
    let file_path = state.config.storage_dir.join(clean_rel);

    // Validação canônica
    let canon_root = match state.config.storage_dir.canonicalize() {
        Ok(r) => r,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Configuração de storage inválida").into_response(),
    };

    let canon_file = match file_path.canonicalize() {
        Ok(f) => f,
        Err(_) => return (StatusCode::NOT_FOUND, "Arquivo não encontrado").into_response(),
    };

    if !canon_file.starts_with(&canon_root) || !canon_file.is_file() {
        return (StatusCode::FORBIDDEN, "Acesso proibido").into_response();
    }

    let mime = mime_guess::from_path(&canon_file)
        .first_or_octet_stream()
        .to_string();

    let file = match tokio::fs::File::open(&canon_file).await {
        Ok(f) => f,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Erro ao abrir arquivo").into_response(),
    };

    let len = match file.metadata().await {
        Ok(m) => m.len(),
        Err(_) => 0,
    };

    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    let mut headers = HeaderMap::new();
    if let Ok(val) = HeaderValue::from_str(&mime) {
        headers.insert(CONTENT_TYPE, val);
    }
    headers.insert(CONTENT_LENGTH, HeaderValue::from(len));

    (StatusCode::OK, headers, body).into_response()
}

const WEB_UI_HTML: &str = r#"<!DOCTYPE html>
<html lang="pt-BR">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Mocambo — Cofre Soberano</title>
  <style>
    :root {
      --bg: #0c0e12;
      --card-bg: #141720;
      --border: #222736;
      --accent: #00e699;
      --accent-hover: #00ffaa;
      --amber: #ffd700;
      --text: #e6edf3;
      --text-muted: #8b949e;
      --danger: #ff5555;
    }
    * { box-sizing: border-box; margin: 0; padding: 0; }
    body {
      background: var(--bg);
      color: var(--text);
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
      min-height: 100vh;
      display: flex;
      flex-direction: column;
    }
    header {
      background: var(--card-bg);
      border-bottom: 1px solid var(--border);
      padding: 1rem 1.5rem;
      display: flex;
      flex-wrap: wrap;
      align-items: center;
      justify-content: space-between;
      gap: 1rem;
    }
    .brand {
      display: flex;
      align-items: center;
      gap: 0.75rem;
      font-weight: 700;
      font-size: 1.25rem;
      letter-spacing: -0.5px;
      color: var(--accent);
    }
    .brand span { color: var(--text); font-weight: 400; font-size: 0.9rem; }
    .search-box {
      display: flex;
      align-items: center;
      background: var(--bg);
      border: 1px solid var(--border);
      border-radius: 8px;
      padding: 0.35rem 0.75rem;
      width: 100%;
      max-width: 480px;
    }
    .search-box input {
      background: transparent;
      border: none;
      outline: none;
      color: var(--text);
      width: 100%;
      font-size: 0.95rem;
    }
    .search-box button {
      background: var(--accent);
      border: none;
      color: #000;
      font-weight: 600;
      padding: 0.35rem 0.75rem;
      border-radius: 6px;
      cursor: pointer;
      transition: background 0.2s;
    }
    .search-box button:hover { background: var(--accent-hover); }
    main {
      flex: 1;
      padding: 1.5rem;
      max-width: 1400px;
      margin: 0 auto;
      width: 100%;
    }
    .breadcrumbs {
      display: flex;
      align-items: center;
      gap: 0.5rem;
      font-size: 0.9rem;
      color: var(--text-muted);
      margin-bottom: 1.25rem;
      font-family: monospace;
    }
    .breadcrumbs a {
      color: var(--accent);
      text-decoration: none;
      cursor: pointer;
    }
    .breadcrumbs a:hover { text-decoration: underline; }
    .grid {
      display: grid;
      grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
      gap: 1rem;
    }
    .item-card {
      background: var(--card-bg);
      border: 1px solid var(--border);
      border-radius: 10px;
      padding: 1rem;
      display: flex;
      flex-direction: column;
      align-items: center;
      text-align: center;
      cursor: pointer;
      transition: transform 0.15s, border-color 0.15s;
    }
    .item-card:hover {
      transform: translateY(-2px);
      border-color: var(--accent);
    }
    .item-icon {
      font-size: 2.5rem;
      margin-bottom: 0.5rem;
    }
    .item-thumb {
      width: 100%;
      height: 120px;
      object-fit: cover;
      border-radius: 6px;
      margin-bottom: 0.5rem;
    }
    .item-name {
      font-size: 0.85rem;
      font-weight: 500;
      word-break: break-word;
      color: var(--text);
      max-height: 2.4em;
      overflow: hidden;
    }
    .item-meta {
      font-size: 0.75rem;
      color: var(--text-muted);
      margin-top: 0.25rem;
    }
    /* Modal Lightbox */
    .modal {
      display: none;
      position: fixed;
      inset: 0;
      background: rgba(0, 0, 0, 0.85);
      backdrop-filter: blur(4px);
      align-items: center;
      justify-content: center;
      z-index: 1000;
      padding: 1.5rem;
    }
    .modal.active { display: flex; }
    .modal-content {
      background: var(--card-bg);
      border: 1px solid var(--border);
      border-radius: 12px;
      max-width: 90vw;
      max-height: 90vh;
      display: flex;
      flex-direction: column;
      overflow: hidden;
    }
    .modal-header {
      padding: 0.75rem 1rem;
      border-bottom: 1px solid var(--border);
      display: flex;
      justify-content: space-between;
      align-items: center;
    }
    .modal-body {
      padding: 1rem;
      overflow: auto;
      display: flex;
      align-items: center;
      justify-content: center;
    }
    .modal-body img {
      max-width: 100%;
      max-height: 75vh;
      border-radius: 6px;
    }
    .close-btn {
      background: transparent;
      border: none;
      color: var(--text-muted);
      font-size: 1.5rem;
      cursor: pointer;
    }
    .close-btn:hover { color: var(--danger); }
  </style>
</head>
<body>
  <header>
    <div class="brand">
      MOCAMBO
      <span>Cofre Soberano & WebDAV</span>
    </div>
    <div class="search-box">
      <input type="text" id="search-input" placeholder="Buscar texto (FTS5) ou imagens semânticas...">
      <button id="search-btn">Buscar</button>
    </div>
  </header>

  <main>
    <div class="breadcrumbs" id="breadcrumbs">/</div>
    <div class="grid" id="file-grid">
      <p style="color: var(--text-muted);">Carregando cofre...</p>
    </div>
  </main>

  <div class="modal" id="preview-modal">
    <div class="modal-content">
      <div class="modal-header">
        <span id="modal-title" style="font-weight: 600;">Visualização</span>
        <button class="close-btn" id="modal-close">&times;</button>
      </div>
      <div class="modal-body" id="modal-body"></div>
    </div>
  </div>

  <script>
    let currentPath = '/';

    function escapeHtml(str) {
      return String(str)
        .replace(/&/g, '&amp;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;')
        .replace(/"/g, '&quot;');
    }

    function formatBytes(bytes) {
      if (!bytes || bytes === 0) return '0 B';
      const k = 1024;
      const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
      const i = Math.floor(Math.log(bytes) / Math.log(k));
      return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
    }

    async function loadDirectory(path) {
      currentPath = path;
      updateBreadcrumbs(path);
      const grid = document.getElementById('file-grid');
      grid.innerHTML = '<p style="color: var(--text-muted);">Carregando...</p>';

      try {
        const res = await fetch('/api/files?path=' + encodeURIComponent(path));
        if (!res.ok) throw new Error('Falha ao carregar diretório');
        const items = await res.json();

        if (items.length === 0) {
          grid.innerHTML = '<p style="color: var(--text-muted); grid-column: 1/-1;">Pasta vazia. Envie arquivos via WebDAV!</p>';
          return;
        }

        grid.innerHTML = '';
        items.forEach(item => {
          const card = document.createElement('div');
          card.className = 'item-card';

          const isImg = item.mime_type.startsWith('image/');
          let iconHtml = '<div class="item-icon">' + (item.is_dir ? '📁' : (isImg ? '🖼️' : '📄')) + '</div>';
          if (isImg) {
            iconHtml = `<img class="item-thumb" src="/raw${encodeURI(item.path)}" loading="lazy" alt="${escapeHtml(item.name)}">`;
          }

          card.innerHTML = `
            ${iconHtml}
            <div class="item-name">${escapeHtml(item.name)}</div>
            <div class="item-meta">${item.is_dir ? 'Pasta' : formatBytes(item.size)}</div>
          `;

          card.onclick = () => {
            if (item.is_dir) {
              loadDirectory(item.path);
            } else {
              openPreview(item);
            }
          };

          grid.appendChild(card);
        });
      } catch (e) {
        grid.innerHTML = '<p style="color: var(--danger);">Erro: ' + escapeHtml(e.message) + '</p>';
      }
    }

    function updateBreadcrumbs(path) {
      const el = document.getElementById('breadcrumbs');
      const parts = path.split('/').filter(Boolean);
      let html = '<a onclick="loadDirectory(\'/\')">raiz</a>';
      let acc = '';
      parts.forEach((p, idx) => {
        acc += '/' + p;
        const target = acc;
        html += ' / <a onclick="loadDirectory(\'' + target + '\')">' + escapeHtml(p) + '</a>';
      });
      el.innerHTML = html;
    }

    function openPreview(item) {
      const modal = document.getElementById('preview-modal');
      const title = document.getElementById('modal-title');
      const body = document.getElementById('modal-body');

      title.textContent = item.name;
      body.innerHTML = '';

      if (item.mime_type.startsWith('image/')) {
        body.innerHTML = `<img src="/raw${encodeURI(item.path)}" alt="${escapeHtml(item.name)}">`;
      } else if (item.mime_type.startsWith('video/')) {
        body.innerHTML = `<video controls autoplay style="max-width: 100%; max-height: 75vh;"><source src="/raw${encodeURI(item.path)}" type="${item.mime_type}"></video>`;
      } else {
        body.innerHTML = `<p style="padding: 2rem;"><a href="/raw${encodeURI(item.path)}" download="${escapeHtml(item.name)}" style="color: var(--accent);">⬇ Baixar ${escapeHtml(item.name)}</a></p>`;
      }

      modal.classList.add('active');
    }

    document.getElementById('modal-close').onclick = () => {
      document.getElementById('preview-modal').classList.remove('active');
    };

    document.getElementById('search-btn').onclick = async () => {
      const q = document.getElementById('search-input').value.trim();
      if (!q) {
        loadDirectory(currentPath);
        return;
      }
      const grid = document.getElementById('file-grid');
      grid.innerHTML = '<p style="color: var(--text-muted);">Buscando nos índices FTS5 e vetores...</p>';
      try {
        const res = await fetch('/api/search?q=' + encodeURIComponent(q));
        if (!res.ok) throw new Error('Falha na busca');
        const results = await res.json();
        if (results.length === 0) {
          grid.innerHTML = '<p style="color: var(--text-muted); grid-column: 1/-1;">Nenhum resultado encontrado para "' + escapeHtml(q) + '".</p>';
          return;
        }
        grid.innerHTML = '';
        results.forEach(r => {
          const card = document.createElement('div');
          card.className = 'item-card';
          card.innerHTML = `
            <div class="item-icon">🔍</div>
            <div class="item-name">${escapeHtml(r.file_path || r.title || 'Arquivo')}</div>
            <div class="item-meta">Score: ${r.score ? r.score.toFixed(2) : '1.0'}</div>
          `;
          card.onclick = () => {
            const rawPath = r.file_path.startsWith('/') ? r.file_path : '/' + r.file_path;
            openPreview({ name: r.file_path, path: rawPath, mime_type: 'application/octet-stream' });
          };
          grid.appendChild(card);
        });
      } catch (e) {
        grid.innerHTML = '<p style="color: var(--danger);">Erro na busca: ' + escapeHtml(e.message) + '</p>';
      }
    };

    document.getElementById('search-input').onkeydown = (e) => {
      if (e.key === 'Enter') document.getElementById('search-btn').click();
    };

    loadDirectory('/');
  </script>
</body>
</html>
"#;

# Especificação Técnica: Motor de Storage POSIX, WebDAV e Checagem de Hash SHA-256

- **Ticket**: [#1](https://codeberg.org/lumenpink/mocambo/issues/1)
- **Status**: Especificado / Fase 3
- **Data**: 2026-10-15
- **Autor**: Lumen Pink & Antigravity

---

## 1. Visão Geral e Propósito

Esta especificação define o núcleo funcional do **Mocambo**, responsável pelo armazenamento de arquivos com transparência POSIX pura (zero lock-in), servidor WebDAV assíncrono em Rust e endpoints REST para validação de integridade por hash SHA-256 (suporte fundamental à política de offload mobile de 15 dias).

---

## 2. Contratos e Arquitetura

### 2.1 Estrutura de Diretórios POSIX no Disco
O diretório raiz de dados é configurável via `MOCAMBO_DATA_DIR` (padrão: `./data/storage`):
```text
/data/storage/{user_id}/
├── Drive/                 # Espaço arbitrário mapeado por clientes WebDAV Desktop
│   ├── Documentos/
│   └── Projetos/
└── Timeline/              # Ingestão cronológica organizada por data
    └── 2026/
        └── 10/
            └── foto.jpg
```
- **Transparência Estrita:** Arquivos gravados são bytes puros no disco, sem compressão proprietária, sem chunks e sem encapsulamento em banco de dados.
- Todo arquivo persistido no disco pode ser aberto diretamente por qualquer ferramenta nativa do sistema operacional (`cat`, `ls`, `rsync`, `ffmpeg`).

### 2.2 Banco de Dados e Metadados (SQLite WAL)
O banco SQLite reside em `./data/mocambo.db` operando obrigatoriamente com:
```sql
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA foreign_keys = ON;
```

#### Esquema Inicial:
```sql
CREATE TABLE IF NOT EXISTS files (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT NOT NULL,
    storage_path TEXT NOT NULL UNIQUE,     -- Caminho relativo dentro do storage
    file_name TEXT NOT NULL,
    file_size INTEGER NOT NULL,
    sha256 TEXT NOT NULL,
    mime_type TEXT NOT NULL,
    created_at INTEGER NOT NULL,            -- Epoch milissegundos
    mtime INTEGER NOT NULL,
    analysis_status TEXT NOT NULL DEFAULT 'pending' -- pending | indexed | failed
);

CREATE INDEX IF NOT EXISTS idx_files_sha256 ON files(sha256);
CREATE INDEX IF NOT EXISTS idx_files_path ON files(storage_path);
```

### 2.3 Motor WebDAV com Interceptador de FS
- Utiliza o crate assíncrono `dav-server` integrado ao runtime `tokio`.
- Envolve o filesystem local em um handler customizado que:
  1. Intercepta requisições `PUT`: ao concluir o stream de gravação no disco, calcula o hash `SHA-256` e registra/atualiza a linha correspondente na tabela `files`.
  2. Intercepta requisições `MOVE`: atualiza o `storage_path` no banco de dados.
  3. Intercepta requisições `DELETE`: remove o registro do banco de dados ao excluir o arquivo físico.

### 2.4 Endpoints REST da API (Axum)
* `GET /health`:
  - Retorna `200 OK` com `{"status": "ok", "version": "0.1.0"}`.
* `HEAD /api/files/check/{hash}`:
  - Se o hash SHA-256 existir no banco e o arquivo estiver presente no disco, retorna `200 OK`.
  - Se o hash não existir ou o arquivo tiver sido removido, retorna `404 Not Found`.
* `GET /api/files/check/{hash}`:
  - Retorna metadados do arquivo em JSON:
    ```json
    {
      "exists": true,
      "sha256": "...",
      "file_name": "...",
      "file_size": 1048576,
      "storage_path": "...",
      "created_at": 1729036800000
    }
    ```

---

## 3. Critérios de Aceite e Verificação Automatizada

1. **Testes Unitários e de Integração:**
   - Teste de gravação WebDAV (`PUT`) verificando persistência crua no disco e geração correta do SHA-256 no banco SQLite.
   - Teste do endpoint `HEAD /api/files/check/{hash}` confirmando `200` para arquivo existente e `404` para hash inexistente.
   - Teste de `GET /health` retornando status operacional.
2. **Qualidade e Estilo:**
   - Zero warnings em `cargo clippy --all-targets -- -D warnings`.
   - Formatação validada com `cargo fmt --check`.
   - 100% de conformidade com GOST 1.1.0 via `cccp check --strict`.

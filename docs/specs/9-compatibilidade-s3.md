# Especificação Técnica — Ticket #9: Compatibilidade S3 (Camada Mínima para Objetos)

## 1. Contexto & Objetivos
O Mocambo opera nativamente sobre armazenamento POSIX local e WebDAV. Para permitir que ferramentas de sincronização, sincronizadores de notas (ex: Obsidian S3 Sync, Logseq) e utilitários de backup (ex: Rclone, Duck, Duplicati, restic) utilizem o cofre como backend de objetos, o servidor deve fornecer uma **camada de compatibilidade S3 mínima**.

### Diretrizes de Arquitetura
1. **Mapeamento Transparente**: O bucket S3 `{bucket}` mapeia diretamente para o diretório raiz `<storage_dir>/{bucket}`. Chaves aninhadas (`a/b/c.png`) são mapeadas para caminhos físicos POSIX (`<storage_dir>/{bucket}/a/b/c.png`).
2. **Integração com Mecanismos de Busca**: Todo upload via S3 PutObject atualiza os metadados na tabela `files` do SQLite, alimentando imediatamente o pipeline assíncrono de extração de texto FTS5 e embeddings de visão CLIP.
3. **Padrão XML S3**: Endpoints de listagem retornam payloads XML compatíveis com `ListBucketResult` (ListObjectsV2).
4. **ETags Baseados em Hash**: ETags retornados seguem o formato `"hash"`.

---

## 2. Endpoints e Contratos REST

```mermaid
flowchart LR
    Client["Cliente S3 (Rclone / Obsidian / CLI)"] -->|PUT /s3/{bucket}/{*key}| PutObj["PutObject (grava POSIX + hash + db)"]
    Client -->|GET /s3/{bucket}/{*key}| GetObj["GetObject (stream + ETag + mime)"]
    Client -->|DELETE /s3/{bucket}/{*key}| DelObj["DeleteObject (remove arquivo + db)"]
    Client -->|GET /s3/{bucket}?list-type=2| ListObj["ListObjectsV2 (XML ListBucketResult)"]
```

### 2.1. PutObject (`PUT /s3/{bucket}/{*key}`)
- Cria diretórios intermediários necessários no storage POSIX.
- Grava bytes via streaming atômico.
- Computa hash SHA-256 e tamanho exato em bytes.
- Registra / atualiza registro na tabela `files`.
- **Resposta**: `200 OK`, headers:
  - `ETag: "{sha256_hash}"`
  - `Content-Length: 0`

### 2.2. GetObject (`GET /s3/{bucket}/{*key}`)
- Localiza arquivo em `<storage_dir>/{bucket}/{clean_key}`.
- Validação de segurança contra Path Traversal (`..`).
- **Resposta**: `200 OK`, headers:
  - `Content-Type: {mime_guess}`
  - `Content-Length: {size}`
  - `ETag: "{sha256_hash}"`
  - Corpo em streaming via `ReaderStream`.
- Se chave não existir: `404 Not Found`.

### 2.3. DeleteObject (`DELETE /s3/{bucket}/{*key}`)
- Remove o arquivo de disco e o registro no banco SQLite.
- **Resposta**: `204 No Content` (idempotente).

### 2.4. ListObjectsV2 (`GET /s3/{bucket}`)
- Parâmetros aceitos via Query String:
  - `prefix`: filtro por prefixo de chave.
  - `max-keys`: limite de chaves retornadas (default 1000).
- **Resposta**: `200 OK`, `Content-Type: application/xml`, corpo contendo XML estruturado com `<ListBucketResult>` e `<Contents>`.

---

## 3. Critérios de Aceite & DD-TDD
- [ ] Teste de integração (`tests/s3_api_test.rs`) cobrindo:
  - `PUT /s3/meubucket/notas/hoje.txt`: PutObject e validação de gravação em disco e ETag.
  - `GET /s3/meubucket/notas/hoje.txt`: GetObject retornando conteúdo idêntico e headers corretos.
  - `GET /s3/meubucket`: ListObjectsV2 retornando XML com a chave `notas/hoje.txt`.
  - `DELETE /s3/meubucket/notas/hoje.txt`: DeleteObject removendo o arquivo e retornando 204.
  - Prevenção contra Directory Traversal em requisições S3 maliciosas (`/s3/bucket/../../etc/passwd`).
- [ ] Implementação do módulo `src/s3.rs` integrado ao router Axum.
- [ ] Conformidade 100% no `cccp check --strict`, zero warnings no Clippy.

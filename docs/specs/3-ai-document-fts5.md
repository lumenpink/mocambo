# Especificação Técnica: Pipeline Assíncrono de Extração Textual e Busca FTS5

- **Ticket**: [#3](https://codeberg.org/lumenpink/mocambo/issues/3)
- **Status**: Especificado / Fase 3
- **Data**: 2026-10-16
- **Autor**: Lumen Pink & Antigravity

---

## 1. Visão Geral e Propósito

Esta especificação define o subsistema assíncrono de ingestão, extração textual e busca profunda no **Mocambo**, eliminando a dependência de serviços externos pesados como Paperless-ngx para documentos de texto e relatórios.

O sistema opera de forma transparente:
1. Arquivos gravados via WebDAV (`Drive/` ou `Timeline/`) são persistidos no disco POSIX puro e registrados na tabela `files` com `analysis_status = 'pending'`.
2. Um canal não-bloqueante (`tokio::sync::mpsc`) notifica o worker de segundo plano.
3. O worker extrai o texto do arquivo (suportando texto plano, Markdown, JSON, CSV e documentos PDF).
4. O conteúdo é indexado na tabela virtual SQLite `files_fts` via **FTS5** com tokenizador `unicode61` (suporte a acentuação e case-insensitive) e ranking BM25.
5. O endpoint `GET /api/search?q=...` expõe buscas instantâneas por palavras-chave com snippets contextuais destacados.

---

## 2. Contratos e Arquitetura

### 2.1 Tabela Virtual FTS5 no SQLite WAL

No SQLite WAL (`data/mocambo.db`), adiciona-se a tabela virtual e triggers/rotinas de indexação:

```sql
CREATE VIRTUAL TABLE IF NOT EXISTS files_fts USING fts5(
    file_id UNINDEXED,
    storage_path UNINDEXED,
    file_name,
    body,
    tokenize = 'unicode61 remove_diacritics 2'
);
```

### 2.2 Estrutura do Pipeline Assíncrono (`src/indexer/mod.rs`)

- **Canal de Notificação:**
  `tokio::sync::mpsc::Sender<IndexTask>` com capacidade de buffer de 512 tarefas.
- **Task:**
  ```rust
  pub struct IndexTask {
      pub storage_path: String,
  }
  ```
- **Extrator Textual (`TextExtractor`):**
  - Textos simples (`text/plain`, `text/markdown`, `application/json`, `text/csv`, etc.): leitura UTF-8 direta com `String::from_utf8_lossy`.
  - Documentos PDF (`application/pdf`): extração de texto estruturado de blocos e streams.
  - Formatos binários sem texto: ignora extração de corpo mantendo apenas o nome do arquivo indexado.
- **Atualização de Estado:**
  - Após indexação, executa `UPDATE files SET analysis_status = 'indexed' WHERE storage_path = ?1`.
  - Em caso de falha irreversível, marca `analysis_status = 'failed'`.

### 2.3 Endpoint de Busca Textual (`GET /api/search`)

- **Requisição:**
  `GET /api/search?q=contrato+aluguel&limit=20&offset=0`
- **Validação:**
  - Se `q` for vazio, retorna HTTP 400 Bad Request.
  - Se `q` contiver sintaxe inválida de FTS5, sanitiza os termos envolvendo em aspas ou tokenizando.
- **Resposta JSON (200 OK):**
  ```json
  {
    "query": "contrato aluguel",
    "total": 1,
    "results": [
      {
        "id": 42,
        "storage_path": "Drive/Documentos/contrato.pdf",
        "file_name": "contrato.pdf",
        "file_size": 1048576,
        "sha256": "3a4f...",
        "mime_type": "application/pdf",
        "snippet": "...firmam o presente <b>contrato</b> de <b>aluguel</b> residencial...",
        "score": -1.452
      }
    ]
  }
  ```

### 2.4 Sincronização em Reescrita e Exclusão

- Ao excluir arquivo no WebDAV: `DELETE FROM files_fts WHERE file_id = ?1`.
- Ao sobrescrever arquivo no WebDAV: o arquivo é enviado novamente ao canal de indexação, que atualiza a entrada correspondente no FTS5.

---

## 3. Critérios de Aceite e Verificação

1. Tabela virtual `files_fts` criada com sucesso e funcional no SQLite WAL.
2. Worker em segundo plano processando tarefas do canal `tokio::sync::mpsc`.
3. Extração correta de texto de arquivos `.txt`, `.md` e documentos de teste.
4. Endpoint `GET /api/search` retornando resultados com correspondência exata, snippet e score.
5. Deleção e atualização refletindo atomicamente na busca FTS5.
6. Testes automatizados passando 100% com `cargo test`.
7. Conformidade total com `cccp check --strict` e zero warnings no Clippy.

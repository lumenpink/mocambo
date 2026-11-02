# Especificação Técnica - Ticket #24
## Deduplicação de Arquivos por Hash e Metadados de Origem de Mídia

### 1. Objetivo
Implementar no núcleo do backend Mocambo o suporte a:
1. Rastreamento e persistência de metadados de origem da mídia (`source` e `original_path`) no banco SQLite e nos fluxos de ingestão.
2. Deduplicação em duas camadas:
   - Em tráfego/rede: endpoint `POST /api/files/dedup` para vincular arquivos idênticos sem retransmitir os bytes do arquivo caso o hash SHA-256 já exista no servidor.
   - Em disco/armazenamento: mecanismo de hardlinks POSIX para deduplicar arquivos com o mesmo hash SHA-256, garantindo uso zero de blocos adicionais de disco.

### 2. Mudanças de Esquema de Dados (SQLite)
Na tabela `files` de `src/db/mod.rs`:
- Adicionar coluna `source TEXT NULL` (ex.: "Camera", "WhatsApp", "Screenshots", "Telegram", "Download").
- Adicionar coluna `original_path TEXT NULL` (caminho relativo de origem no cliente, ex.: "WhatsApp/Media/WhatsApp Images/IMG-001.jpg").
- Adicionar índice `idx_files_source ON files(source)`.
- Atualizar a estrutura `FileRecord` em Rust com `pub source: Option<String>` e `pub original_path: Option<String>`.

### 3. Endpoints e Contratos de API
1. `HEAD /api/files/check/:sha256`:
   - Retorna `200 OK` se o hash já existir no banco da usuária.
   - Retorna `404 Not Found` se o hash for inédito.
2. `GET /api/files/check/:sha256`:
   - Retorna `200 OK` com JSON contendo `exists: true`, metadados completos (`file_name`, `sha256`, `source`, `original_path`, etc.).
   - Retorna `404 Not Found` caso inexistente.
3. `POST /api/files/dedup`:
   - Payload:
     ```json
     {
       "sha256": "<hash>",
       "storage_path": "/dav/Timeline/2026/11/IMG.jpg",
       "file_name": "IMG.jpg",
       "source": "WhatsApp",
       "original_path": "Android/media/com.whatsapp/..."
     }
     ```
   - Se o hash existir: cria hardlink para o arquivo de destino (se não existir no caminho), insere/atualiza o registro em `files` com os metadados de origem, enfileira para indexação e retorna `201 Created`.
   - Se o hash não existir: retorna `404 Not Found`.
4. `GET /api/files/sources`:
   - Retorna lista de categorias de origem presentes no banco: `["Camera", "WhatsApp", "Screenshots", ...]`.

### 4. Deduplicação no Storage WebDAV
No módulo `src/webdav/mod.rs`:
- Suporte a cabeçalhos HTTP `X-Mocambo-Source` e `X-Mocambo-Original-Path`.
- Ao finalizar a escrita de um arquivo (`InterceptingFile::flush`), verificar se o hash gerado já reside no banco em outro caminho. Em caso positivo, substitui o arquivo recém-gravado por um `std::fs::hard_link` com o arquivo já existente.

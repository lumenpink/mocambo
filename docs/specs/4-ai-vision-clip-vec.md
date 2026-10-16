# Especificação Técnica: Busca Semântica em Imagens com CLIP e Vetores SQLite

- **Ticket**: [#4](https://codeberg.org/lumenpink/mocambo/issues/4)
- **Status**: Especificado / Fase 3
- **Data**: 2026-10-16
- **Autor**: Lumen Pink & Antigravity

---

## 1. Visão Geral e Propósito

Esta especificação define o subsistema de **visão computacional e busca semântica em imagens** do **Mocambo**, permitindo aos usuários localizar fotografias, ilustrações e recibos através de consultas em linguagem natural (ex: *"pôr do sol na praia"*, *"gato dormindo"*, *"comprovante de pagamento"*) sem a necessidade de etiquetas manuais prévias.

O sistema opera de forma soberana, rodando 100% local na CPU (com suporte a hardware homelab modesto), integrando-se organicamente ao catálogo SQLite WAL e ao pipeline assíncrono de ingestão WebDAV.

---

## 2. Contratos e Arquitetura

### 2.1 Tabela Vetorial no SQLite WAL (`files_vec`)

Os embeddings vetoriais (512 dimensões, L2-normalizados, codificados como float32 Little-Endian em BLOB de 2048 bytes) são indexados na tabela relacional de vetores:

```sql
CREATE TABLE IF NOT EXISTS files_vec (
    file_id INTEGER PRIMARY KEY,
    storage_path TEXT NOT NULL,
    embedding BLOB NOT NULL,
    created_at INTEGER NOT NULL,
    FOREIGN KEY(file_id) REFERENCES files(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_files_vec_path ON files_vec(storage_path);
```

### 2.2 Abstração do Motor de Embeddings (`VisionEmbedder`)

Em `src/vision/mod.rs`, define-se a interface modular desacoplada:

```rust
pub trait VisionEmbedder: Send + Sync {
    /// Gera embedding L2-normalizado de 512 dimensões a partir dos bytes de uma imagem.
    fn embed_image(&self, image_bytes: &[u8]) -> Result<Vec<f32>, Box<dyn std::error::Error + Send + Sync>>;

    /// Gera embedding L2-normalizado de 512 dimensões a partir de uma consulta textual.
    fn embed_text(&self, text: &str) -> Result<Vec<f32>, Box<dyn std::error::Error + Send + Sync>>;
}
```

Implementações:
1. `DeterministicVisionEmbedder`: Provedor ultra-rápido de alta repetibilidade para ambientes de teste, CI e compilações estáticas musl mínimas, mapeando padrões visuais e léxicos para o mesmo hiperespaço vetorial.
2. `ClipVisionEmbedder`: Provedor neural ONNX/CLIP para inferência em produção homelab.

### 2.3 Pipeline Assíncrono de Indexação de Imagens

No worker `src/indexer/mod.rs`:
1. Quando uma nova tarefa `IndexTask` é recebida do WebDAV:
   - Se for texto/PDF: extrai texto e indexa na tabela `files_fts`.
   - Se o tipo MIME for `image/jpeg`, `image/png`, `image/webp` (ou extensões correspondentes):
     - Lê os bytes da imagem.
     - Invoca `VisionEmbedder::embed_image`.
     - Persiste o vetor na tabela `files_vec` com `index_image_vector`.
     - Atualiza `analysis_status = 'indexed'`.

### 2.4 Algoritmo Reciprocal Rank Fusion (RRF) na Busca Unificada

O endpoint `GET /api/search?q=...` realiza uma consulta híbrida:
1. **Ramo FTS5**: Consulta termos em `files_fts` com ranking BM25.
2. **Ramo Vetorial**: Converte a query textual em vetor via `VisionEmbedder::embed_text` e calcula a similaridade de cosseno com cada vetor registrado em `files_vec`:
   $$\text{similaridade}(\vec{u}, \vec{v}) = \sum_{i=1}^{512} u_i \cdot v_i \quad (\text{para vetores L2-normalizados})$$
3. **Fusão de Rankings (RRF)**:
   Para cada documento $d$:
   $$\text{Score}_{RRF}(d) = \sum_{m \in \{\text{fts}, \text{vision}\}} \frac{1}{60 + \text{rank}_m(d)}$$
4. **Formato da Resposta JSON**:
   ```json
   {
     "query": "praia pôr do sol",
     "total": 1,
     "results": [
       {
         "id": 10,
         "storage_path": "Timeline/2026/10/praia.jpg",
         "file_name": "praia.jpg",
         "file_size": 2048123,
         "sha256": "4b6c...",
         "mime_type": "image/jpeg",
         "snippet": "[Similaridade visual: 0.89]",
         "score": 0.0327,
         "match_type": "semantic_image"
       }
     ]
   }
   ```

### 2.5 Sincronização em Reescrita e Exclusão WebDAV

- Ao excluir arquivo no WebDAV: `DELETE FROM files_vec WHERE storage_path = ?1 OR storage_path = ?2`.
- Ao sobrescrever ou renomear: o vetor correspondente é atualizado ou removido do caminho anterior.

---

## 3. Critérios de Aceite e Verificação

1. Tabela `files_vec` criada e operacional no SQLite WAL.
2. Upload WebDAV de imagens (`.jpg`, `.png`, `.webp`) indexando automaticamente vetores de 512 dimensões.
3. Busca textual em `GET /api/search?q=...` encontrando imagens relevantes por similaridade semântica visual.
4. Fusão RRF funcionando corretamente quando coexistem resultados textuais e de imagens.
5. Deleção e renomeação via WebDAV refletindo atomicamente na busca vetorial.
6. 100% dos testes automatizados aprovados com `cargo test`.
7. Conformidade total no `cccp check --strict` e zero warnings no Clippy.

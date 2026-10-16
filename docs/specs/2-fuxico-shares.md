# Especificação Técnica: Integração de Compartilhamento com Fuxico e Protocolo Bidirecional

- **Ticket**: [#2](https://codeberg.org/lumenpink/mocambo/issues/2)
- **Status**: Especificado / Fase 3
- **Data**: 2026-10-16
- **Autor**: Lumen Pink & Antigravity

---

## 1. Visão Geral e Propósito

Esta especificação define o subsistema de compartilhamento externo do **Mocambo**, operando sob a diretriz estrita de segurança de **não exposição pública direta**:
- O Mocambo permanece 100% isolado na rede interna / VPN privada (Tailscale/WireGuard).
- Toda URL pública de compartilhamento é delegada ao **Fuxico** (`2lp.in` / `fuxico.lohn.in`), que atua como porta de entrada e gateway de pré-download, zine e streaming reverso autenticado.
- Nenhum arquivo é duplicado em disco. O Fuxico realiza streaming reverso autenticado diretamente do Mocambo a partir do endpoint interno `/api/internal/stream/:token`.
- Notificações de expiração, revogação e deleção mútua mantêm os dois sistemas perfeitamente sincronizados.

---

## 2. Contratos e Arquitetura

### 2.1 Esquema de Banco de Dados (`shares`)

No SQLite WAL (`data/mocambo.db`), adiciona-se a tabela `shares`:

```sql
CREATE TABLE IF NOT EXISTS shares (
    id TEXT PRIMARY KEY,
    token TEXT NOT NULL UNIQUE,            -- Token interno moca_tok_<hex>
    user_id TEXT NOT NULL,
    storage_path TEXT NOT NULL,            -- Caminho POSIX do arquivo no storage
    fuxico_slug TEXT NOT NULL,             -- Slug público gerado no Fuxico
    fuxico_url TEXT NOT NULL,              -- URL pública completa (ex: https://2lp.in/abc123)
    created_at INTEGER NOT NULL,           -- Epoch em segundos
    expires_at INTEGER NULL,               -- Epoch em segundos de expiração ou NULL
    status TEXT NOT NULL DEFAULT 'active'  -- active | revoked | expired
);

CREATE INDEX IF NOT EXISTS idx_shares_token ON shares(token);
CREATE INDEX IF NOT EXISTS idx_shares_storage_path ON shares(storage_path);
CREATE INDEX IF NOT EXISTS idx_shares_fuxico_slug ON shares(fuxico_slug);
```

### 2.2 Cliente HTTP Fuxico (`src/fuxico/client.rs`)

Configuração necessária em `Config` (`src/config.rs`):
- `fuxico_base_url`: URL do Fuxico (ex: `https://2lp.in` ou `https://fuxico.lohn.in`).
- `fuxico_api_token`: Token mestre ou de usuário autorizado no Fuxico.
- `mocambo_internal_url`: URL base acessível pelo Fuxico via VPN (ex: `http://100.64.0.2:8080`).

#### Criação de Compartilhamento (`POST /api/mocambo/share` no Fuxico):
- **Payload:**
  ```json
  {
    "file_name": "relatorio.pdf",
    "file_size": 1024,
    "mime_type": "application/pdf",
    "mocambo_token": "moca_tok_abcdef123456",
    "mocambo_stream_url": "http://100.64.0.2:8080/api/internal/stream/moca_tok_abcdef123456",
    "ttl": "24h"
  }
  ```
- **Resposta do Fuxico:**
  ```json
  {
    "slug": "abc123",
    "url": "https://2lp.in/abc123",
    "x_token": "token_remocao"
  }
  ```

#### Revogação de Compartilhamento (`DELETE /api/mocambo/share/{slug}` no Fuxico):
- Header `Authorization: Bearer <fuxico_api_token>`.
- Remove o compartilhamento imediatamente no Fuxico.

### 2.3 Endpoint Interno de Streaming (`GET /api/internal/stream/:token`)

- Rota interna autenticada pelo próprio `:token`.
- Consulta a tabela `shares`:
  - Se não existir ou `status != 'active'`, retorna HTTP 404 / 410.
  - Se `expires_at` for não-nulo e menor que `now()`, atualiza status para `expired` e retorna HTTP 410 Gone.
- Localiza o arquivo POSIX correspondente via `storage_path`.
- Realiza streaming chunked com Axum e Tokio `ReaderStream` com cabeçalhos adequados (`Content-Type`, `Content-Length`, `Content-Disposition`).

### 2.4 Endpoint de Criação e Revogação no Mocambo

- `POST /api/shares`:
  - Recebe `{ "storage_path": "Drive/relatorio.pdf", "ttl": "24h" }`.
  - Dispara a criação via `FuxicoClient`.
  - Salva em `shares` e retorna a URL pública gerada no Fuxico.
- `DELETE /api/shares/:token` ou `DELETE /api/shares/by-slug/:slug`:
  - Notifica o Fuxico via `DELETE /api/mocambo/share/{slug}`.
  - Atualiza o banco local para `revoked`.

### 2.5 Sincronização em Exclusão Física no Storage

- No `InterceptingFs` do WebDAV (`src/webdav/mod.rs`), ao executar `DELETE` ou `MOVE` de um arquivo que possui shares ativos:
  - Todas as shares associadas a `storage_path` são revogadas no Fuxico e marcadas como `revoked` no banco de dados.

---

## 3. Critérios de Aceite e Verificação

1. Tabela `shares` no banco de dados SQLite com índices apropriados.
2. Cliente `FuxicoClient` modular e testável (com suporte a mock/in-memory ou servidor local de teste).
3. Endpoint `GET /api/internal/stream/:token` entregando bytes puros via streaming sem carregar na memória.
4. Endpoint `POST /api/shares` criando a share e retornando o link público do Fuxico.
5. Endpoint `DELETE /api/shares/:token` revogando no Fuxico e no banco local.
6. Testes automatizados passando 100% com `cargo test`.
7. Conformidade total com `cccp check --strict` e zero warnings no Clippy.

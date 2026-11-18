# Especificação Técnica — Ticket #35: Propagação de Remote-User do Authelia no WebDAV e Dedup

- **Ticket**: [#35](https://gambi.2lp.in/lumen/mocambo/issues/35)
- **Status**: Fase 3 - Especificação
- **Data**: 2026-11-18
- **Autor**: Lumen Pink & Antigravity

---

## 1. Contexto e Motivação (Guardiã da Natasha)

O Mocambo foi configurado sob o domínio canônico `mocambo.lohn.in` com proteção perimetral via Caddy e autenticação centralizada no Authelia (v4.39 / LLDAP).
No fluxo de Forward Auth do Authelia:
- Clientes navegadores recebem desafio SSO via cookie/redirecionamento.
- Clientes WebDAV (Zotero, rclone, sistemas operacionais) recebem desafio HTTP Basic Auth (`401 Unauthorized` com `WWW-Authenticate: Basic realm="Mocambo Vault"`).
- O Authelia valida as credenciais contra a árvore do LLDAP (`uid=...,ou=people`).
- Uma vez autenticada a requisição, o Caddy injeta os cabeçalhos confiáveis da sessão, notadamente `Remote-User: <login>` (ex.: `Remote-User: lumen`).

Anteriormente, o `user_id` no backend do Mocambo era hardcoded como `"default"` tanto no `InterceptingFs` quanto nos registros gravados na tabela `files` do SQLite.

Esta especificação define a leitura dinâmica do cabeçalho `Remote-User` (com fallback seguro para `X-Forwarded-User` e `"default"`) em:
1. Requisições WebDAV de upload (`PUT` / `flush` / gravação de arquivo físico e metadados).
2. Endpoint REST de deduplicação (`POST /api/files/dedup`).

Dessa forma, arquivos enviados pelo Zotero ou pelo app móvel são devidamente associados ao usuário autenticado no banco de dados, preparando o sistema para visões multiusuário sem quebrar ambientes de testes locais desacoplados.

---

## 2. Contratos e Arquitetura

### 2.1 WebDAV Interceptador (`InterceptingFs` e `InterceptingFile`)
O mapa de metadados pendentes (`PendingMetadataMap`) armazena temporariamente metadados da requisição HTTP até a finalização do stream no disco (`flush()`):
- **Antes**: `HashMap<String, (String, Option<String>)>` guardando `(source, original_path)`.
- **Depois**: `HashMap<String, FileRequestMetadata>` guardando:
  ```rust
  pub struct FileRequestMetadata {
      pub user_id: String,
      pub source: Option<String>,
      pub original_path: Option<String>,
  }
  ```

Ao interceptar requisições `PUT` no router fallback do Axum:
1. Extrai `Remote-User` (ou `X-Forwarded-User`), normalizando espaços e caindo em `"default"` caso ausente.
2. Extrai cabeçalhos adicionais opcionais do cliente móvel (`x-mocambo-source`, `x-mocambo-original-path`).
3. Registra na tabela de metadados pendentes sob o path normalizado da requisição WebDAV.
4. No método `flush()`, ao calcular o SHA-256 e persistir na tabela `files`, consome os metadados pendentes e grava o `user_id` correspondente.

### 2.2 Endpoint REST de Deduplicação (`POST /api/files/dedup`)
Na rota `POST /api/files/dedup`:
- Inspeciona os cabeçalhos HTTP da requisição (`Remote-User` / `X-Forwarded-User`).
- Caso presente, grava o hardlink e o registro na tabela `files` associado a esse `user_id`, e não `"default"`.

### 2.3 Resiliência e Fallbacks
- Se nenhuma autenticação perimetral estiver presente (ex.: testes unitários locais sem proxy), o valor padrão `"default"` garante total retrocompatibilidade e hermetismo de execução.

---

## 3. Critérios de Aceite DD-TDD

- [ ] **Red Test:** Teste automatizado de integração em `tests/auth_remote_user_test.rs` simulando:
  - Upload WebDAV (`PUT /dav/zotero/test_paper.pdf`) com cabeçalho `Remote-User: lumen`.
  - Verificação no SQLite de que a linha em `files` contém `user_id = "lumen"`.
  - Requisição de `POST /api/files/dedup` com `Remote-User: lumen` registrando `user_id = "lumen"`.
  - Upload sem cabeçalho `Remote-User` registrando com `user_id = "default"`.
- [ ] **Green Implementation:** Implementar em `src/webdav/mod.rs`, `src/app.rs` e `src/api/mod.rs`.
- [ ] **Quality Gate:** Zero warnings em clippy, formatação fmt impecável e suite completa de testes aprovada.

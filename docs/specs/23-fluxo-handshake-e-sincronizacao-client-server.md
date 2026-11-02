# Especificação Técnica — Ticket #23: Fluxo de Handshake e Sincronização Client-Server

## 1. Contexto & Objetivos
Com o servidor Mocambo implantado no `obizero` sob `mocambo-server.2lp.in` e a tela de configurações e permissões locais prontas no client, este ticket implementa o fluxo completo de handshake e sincronização ponta-a-ponta entre o app móvel e o servidor.

### Diretrizes de Implementação
1. **Handshake e Verificação Prévia:** Antes de qualquer tentativa de transferência, o client valida o endpoint `/health` via `ApiClient.handshake()`. Caso o servidor esteja inacessível ou as configurações incompletas, a usuária é informada e a operação é abortada sem perda de dados.
2. **Autenticação:** O client envia cabeçalhos `Authorization` em todas as requisições API e WebDAV quando um token/senha estiver presente.
3. **Sincronização em Lote (`syncPendingMedia`):** Upload estruturado via WebDAV PUT (`/dav/Timeline/YYYY/MM/{fileName}`) de itens com status `pending`, atualizando para `synced` após resposta 2xx.
4. **Ciclo Completo com Offload Seguro:** Após sincronização bem-sucedida, acionamento do `OffloadManager` que consulta integridade criptográfica no backend via `HEAD /api/files/check/{hash}` antes de expurgar mídias locais com idade >= retenção definida.
5. **Feedback Visual:** Notificações em tempo real com contagem de arquivos enviados, itens liberados e tratamento de erros de rede.

---

## 2. Critérios de Aceite & Verificação DD-TDD
- [ ] Teste automatizado de integração em `tests/mobile_contract_test.rs` simulando o handshake `/health`, upload WebDAV de mídia e checagem de offload.
- [ ] Método `handshake()` no `ApiClient`.
- [ ] Método `syncPendingMedia()` no `WebDavSyncService`.
- [ ] Fluxo integrado no `main.dart` e acionável via `GalleryPage`.
- [ ] Conformidade 100% no Quality Gate.

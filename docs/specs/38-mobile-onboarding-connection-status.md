# Especificação Técnica — Ticket #38: Onboarding de Conexão e Status Visual de Sincronização

- **Ticket**: [#38](https://gambi.2lp.in/lumen/mocambo/issues/38)
- **Status**: Concluído (Fase 9)
- **Data**: 2026-11-19
- **Autor**: Lumen Pink & Antigravity

---

## 1. Contexto e Motivação (Guardiã da Natasha)

No client mobile do Mocambo, após abrir o app pela primeira vez, não havia nenhuma indicação visual de que o servidor precisava ser conectado, deixando a impressão de que a aplicação estava "mockada" ou isolada.
Além disso, durante a sincronização de fotos e vídeos para o WebDAV (`/dav/Timeline/...`), não havia feedback em tempo real sobre o status da conexão com `mocambo.lohn.in` nem indicador de progresso durante uploads em lote.

Esta especificação resolve:
1. **Banner de Onboarding na Timeline:** Exibição de um card acolhedor no topo da tela inicial quando o servidor não estiver configurado, com chamada para ação ("Configurar Servidor") que direciona diretamente à tela de Ajustes.
2. **Barra de Status do Vault:** Indicador visual de conectividade na Timeline com três estados:
   - *Desconectado / Não configurado* (âmbar)
   - *Conectado e Saudável* (verde, exibindo o host ativo)
   - *Sincronizando Mídias* (azul com progresso `X de Y` e animação de sincronização)
3. **Aprimoramento do Handshake e Feedback:** Tratamento detalhado de erros HTTP (401 Unauthorized do Authelia vs 502/Network Error) com feedback claro para o usuário.

---

## 2. Contratos e Arquitetura

### 2.1 Modelo de Estado de Sincronização (`SyncState`)
Em `mobile/lib/views/gallery_page.dart`:
- Parâmetros recebidos:
  - `final AppConfig config;`
  - `final bool isSyncing;`
  - `final String? syncProgressMessage;`
  - `final VoidCallback? onOpenSettings;`
- Exibir card de onboarding no topo de `body` quando `!config.isConfigured`:
  - Ícone de nuvem, mensagem explicativa sobre a autonomia dos dados e botão `Configurar Servidor`.
- Exibir chip/faixa compacta de status quando configurado, indicando o estado atual (`Pronto`, `Sincronizando...`).

### 2.2 Controle de Fluxo em `main.dart`
- Adicionar no `_MocamboAppState`:
  - `bool _isSyncing = false;`
  - `String? _syncMessage;`
- No `_handleSyncWithServer()`:
  - Atualizar `_isSyncing = true;` e `_syncMessage = 'Conectando ao servidor...';`.
  - Passar callback `onProgress: (current, total)` para `_webDavSync.syncAllPending(...)`.
  - Notificar ao concluir com quantidade de sucessos e falhas.
  - No callback `onOpenSettings`, alternar a navegação para a aba de Configurações (`_currentIndex = 2`).

---

## 3. Critérios de Aceite DD-TDD

- [x] **Red Test:** Teste em `tests/mobile_contract_test.rs` validando que:
  - `gallery_page.dart` aceita `config`, `isSyncing` e `onOpenSettings`.
  - `gallery_page.dart` contém o banner de onboarding "Configurar Servidor".
  - `main.dart` repassa o estado de sincronização e navegação de onboarding para a `GalleryPage`.
- [x] **Green Implementation:** Implementar em `mobile/lib/views/gallery_page.dart` e `mobile/lib/main.dart`.
- [x] **Quality Gate:** Suite de testes em Rust aprovada, `cargo fmt` e `cargo clippy` sem warnings.

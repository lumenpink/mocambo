# Especificação Técnica — Ticket #36: Persistência de Ajustes e Autenticação HTTP Basic no Client Móvel

- **Ticket**: [#36](https://gambi.2lp.in/lumen/mocambo/issues/36)
- **Status**: Concluído (Fase 9)
- **Data**: 2026-11-19
- **Autor**: Lumen Pink & Antigravity

---

## 1. Contexto e Motivação (Guardiã da Natasha)

No client Flutter (`mobile/`), a classe `AppConfig` iniciava vazia em memória RAM no `main.dart`, sem carregar dados salvos em disco no `initState()`, e qualquer alteração salva na `SettingsPage` era perdida ao fechar o app.
Adicionalmente, com a ativação de `mocambo.lohn.in` sob o Caddy com Forward Auth no Authelia/LLDAP, requisições com `Authorization: Bearer <token>` são rejeitadas ou redirecionadas para o login web do SSO. O padrão canônico para clientes headless (Zotero, rclone, app móvel) comunicando-se com o perímetro do Authelia é **HTTP Basic Auth**:
`Authorization: Basic <base64(username:password)>`

Esta especificação resolve:
1. **Persistência Local Automática:** Carregamento de `AppConfig` no startup do app e salvamento durável no disco ao submeter o formulário de ajustes.
2. **Resolução de Caminho Hermético:** Método utilitário `defaultConfigFile()` que localiza o diretório de dados do aplicativo no Android (`/data/user/0/in.twolp.mocambo/app_flutter/config.json`) e fallback para `$HOME/.config/mocambo/config.json` no Desktop/Linux.
3. **Protocolo HTTP Basic:** Atualização de `ApiClient` e `WebDavSyncService` para enviar `Authorization: Basic <base64>` sempre que `username` e `password` estiverem presentes.
4. **URL Padrão Sugerida:** Inclusão de `https://mocambo.lohn.in` como sugestão/placeholder na interface de configurações.

---

## 2. Contratos e Arquitetura

### 2.1 Armazenamento de Configuração (`ConfigStore`)
Em `mobile/lib/config/app_config.dart`:
- Adicionar o método estático `defaultConfigFile()`:
  - Detecta `/data/user/0/in.twolp.mocambo/app_flutter` ou `/data/data/in.twolp.mocambo/app_flutter`.
  - Fallback para `$HOME/.config/mocambo/config.json` ou `./config.json`.
- `ConfigStore.load()` e `ConfigStore.save(config)`.

### 2.2 Ciclo de Vida em `main.dart`
- No `_MocamboAppState`:
  - `_configStore` inicializado com o caminho retornado por `defaultConfigFile()`.
  - No `initState()`, dispara leitura assíncrona de `_configStore.load()`.
  - Ao concluir, invoca `setState()` com o `AppConfig` carregado e reinicializa os serviços (`ApiClient`, `WebDavSyncService`, `OffloadManager`).
  - No `_updateConfig(newConfig)`, persiste imediatamente chamando `_configStore.save(newConfig)`.

### 2.3 Cabeçalho de Autenticação (`Basic Auth`)
Em `mobile/lib/services/api_client.dart` e `mobile/lib/services/webdav_sync.dart`:
```dart
if (config.password.isNotEmpty && config.username.isNotEmpty) {
  final credentials = '${config.username}:${config.password}';
  final encoded = base64.encode(utf8.encode(credentials));
  headers['Authorization'] = 'Basic $encoded';
}
```

---

## 3. Critérios de Aceite DD-TDD

- [x] **Red Test:** Teste em `tests/mobile_contract_test.rs` validando que:
  - `app_config.dart` possui `defaultConfigFile` e `ConfigStore`.
  - `api_client.dart` e `webdav_sync.dart` utilizam `Basic ` na montagem do cabeçalho de autenticação.
  - `main.dart` invoca carregamento durável no `initState` e salvamento no `_updateConfig`.
- [x] **Green Implementation:** Implementar em `mobile/lib/config/app_config.dart`, `mobile/lib/services/api_client.dart`, `mobile/lib/services/webdav_sync.dart`, `mobile/lib/views/settings_page.dart` e `mobile/lib/main.dart`.
- [x] **Quality Gate:** Suite de testes em Rust aprovada, `cargo fmt` e `cargo clippy` sem warnings.

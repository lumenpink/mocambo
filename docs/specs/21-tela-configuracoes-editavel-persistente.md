# Especificação Técnica — Ticket #21: Tela de Configurações Editável, Persistente e Sem Defaults Preenchidos

## 1. Contexto & Objetivos
Anteriormente, o aplicativo mobile possuía uma tela de configurações puramente estática (`StatelessWidget`) com valores padrão fictícios hardcoded (`http://127.0.0.1:8080` e usuário `default`).

O objetivo deste ticket é:
1. **Remoção de Pré-configurações:** A interface não deve conter nenhum valor pré-configurado. O estado inicial da aplicação deve ser limpo/vazio (`serverUrl: ''`, `username: ''`, `password: ''`), exigindo que a usuária configure conscientemente seu servidor.
2. **Formulário Editável:** Implementar campos de entrada (`TextField`) para:
   - URL do Servidor (ex: `https://mocambo-server.2lp.in`)
   - Nome de Usuário
   - Senha / Token de Autenticação
   - Janela de Retenção (dias antes do offload)
3. **Teste de Conectividade:** Fornecer ação "Testar Conexão" que consulta o endpoint `/health` do servidor configurado e exibe feedback imediato de sucesso ou erro descritivo.
4. **Persistência Local:** Salvar e carregar as configurações em armazenamento local (`ConfigStore`) para que persistam entre reinicializações do app.
5. **Indicação de Status:** Alerta visual na aplicação caso o servidor ainda não esteja configurado.

---

## 2. Contratos & Modelo de Dados

### `AppConfig`
- `serverUrl: String` (default `''`)
- `username: String` (default `''`)
- `password: String` (default `''`)
- `retentionDays: int?` (default `null`)
- `bool get isConfigured => serverUrl.trim().isNotEmpty && username.trim().isNotEmpty;`
- `String get healthUrl => '$cleanBaseUrl/health';`
- Métodos `toJson()` e `fromJson()`.

### `ConfigStore`
- `Future<AppConfig> loadConfig()`
- `Future<void> saveConfig(AppConfig config)`

---

## 3. Critérios de Aceite & Verificação DD-TDD
- [ ] Teste automatizado em `tests/mobile_contract_test.rs` validando que:
  - `AppConfig` não possui defaults fixos de URL ou usuário.
  - `SettingsPage` declara campos editáveis (`TextFormField` ou `TextField`) e botões de testar conexão e salvar.
  - `ConfigStore` para persistência local está implementado.
- [ ] Testes de widget no Flutter validando a interação do formulário.
- [ ] Conformidade 100% no Quality Gate.

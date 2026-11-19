# Especificação Técnica — Ticket #37: Solicitação de Permissões em Runtime e Scan Abrangente da Galeria

- **Ticket**: [#37](https://gambi.2lp.in/lumen/mocambo/issues/37)
- **Status**: Concluído (Fase 9)
- **Data**: 2026-11-19
- **Autor**: Lumen Pink & Antigravity

---

## 1. Contexto e Motivação (Guardiã da Natasha)

No client Android do Mocambo, usuários relataram que o app não exibia nenhuma foto ou vídeo na timeline e parecia "mockado".
A causa raiz identificada compreende três fatores:
1. **Ausência de Pedido de Permissão em Runtime no Android:** O Android 13+ (API 33+) exige a concessão explícita de `READ_MEDIA_IMAGES` e `READ_MEDIA_VIDEO` via diálogo do sistema operacional em tempo de execução. Sem isso, chamadas de sistema no filesystem retornam `EACCES (Permission Denied)` silencioso.
2. **Scan Incompleto com Interrupção Prematura:** O método `_handleScanMedia` possuía uma instrução `break` no primeiro diretório com arquivos e listava apenas `/sdcard/DCIM/Camera` e `/sdcard/Pictures`, ignorando os caminhos canônicos do Android `/storage/emulated/0/...` (incluindo Downloads, WhatsApp e Screenshots).
3. **Falta de Auto-Scan no Startup:** O aplicativo não disparava o scan da galeria ao ser inicializado após carregar o estado, dependendo de ação manual não intuitiva.

Esta especificação resolve tais gargalos com arquitetura limpa, soberana e livre de dependências externas frágeis.

---

## 2. Contratos e Arquitetura

### 2.1 Canal Nativo de Permissões (`in.twolp.mocambo/permissions`)
Em `mobile/android/app/src/main/kotlin/in/twolp/mocambo/MainActivity.kt`:
- Registrar `MethodChannel` sob o canal canônico `"in.twolp.mocambo/permissions"`.
- Implementar `requestStoragePermissions()` e `hasStoragePermissions()`:
  - No Android 13+ (Tiramisu / API 33+): solicita `Manifest.permission.READ_MEDIA_IMAGES` e `Manifest.permission.READ_MEDIA_VIDEO`.
  - No Android 12 ou inferior: solicita `Manifest.permission.READ_EXTERNAL_STORAGE`.
  - Retorna `true` se as permissões estiverem concedidas e trata o callback `onRequestPermissionsResult`.

### 2.2 Serviço de Permissões Flutter (`PermissionService`)
Em `mobile/lib/services/permission_service.dart`:
- Fornecer método `Future<bool> requestStoragePermissions()`.
- Em ambiente não-Android (Linux desktop, testes), retorna `true` imediatamente.
- No Android, invoca o canal nativo com fallback seguro.

### 2.3 Varredura Abrangente de Mídia (`main.dart` e `media_scanner.dart`)
- Expandir candidatos de diretórios no Android para:
  - `/storage/emulated/0/DCIM/Camera`
  - `/storage/emulated/0/DCIM`
  - `/storage/emulated/0/Pictures`
  - `/storage/emulated/0/Download`
  - `/sdcard/DCIM/Camera`
  - `/sdcard/Pictures`
  - `/sdcard/Download`
  - `$HOME/Pictures` (Desktop)
  - `./test_media`
- Remover o `break` para escanear e unificar todos os diretórios encontrados.
- Invocar `_handleScanMedia()` logo após o carregamento das configurações (`_loadConfig`).
- Ordenar a timeline por data de criação (`createdAt` decrescente).

---

## 3. Critérios de Aceite DD-TDD

- [x] **Red Test:** Teste em `tests/mobile_contract_test.rs` validando que:
  - `MainActivity.kt` implementa o canal `"in.twolp.mocambo/permissions"` com suporte a runtime permissions.
  - `mobile/lib/services/permission_service.dart` existe e possui `requestStoragePermissions`.
  - `mobile/lib/main.dart` solicita permissões antes de escanear e lista múltiplos diretórios canônicos (`/storage/emulated/0`).
- [x] **Green Implementation:** Implementar em `MainActivity.kt`, `permission_service.dart`, `media_scanner.dart` e `main.dart`.
- [x] **Quality Gate:** Suite de testes em Rust aprovada, `cargo fmt` e `cargo clippy` sem warnings.

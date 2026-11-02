# Especificação Técnica - Ticket #25
## Detecção de Pasta de Origem e Deduplicação Inteligente de Upload no App Mobile

### 1. Objetivo
Implementar no aplicativo Flutter (Mocambo Mobile):
1. Classificação e rastreamento da pasta de origem da mídia (`source` e `originalRelativePath`) no scanner de mídia local (`MediaScanner`).
2. Deduplicação inteligente de upload: antes de transmitir arquivos pesados pela rede móvel ou Wi-Fi, o app consulta se o hash SHA-256 já existe no servidor. Se existir, registra o vínculo via `/api/files/dedup` e ignora a transferência dos bytes binários, economizando bateria e franquia de dados.
3. Envio dos cabeçalhos de contexto de procedência (`X-Mocambo-Source` e `X-Mocambo-Original-Path`) nos uploads WebDAV regulares.
4. Filtro visual por origem na tela de Galeria (`GalleryPage`) com seleção por Chips (Todas, Câmera, WhatsApp, Screenshots, etc.).

### 2. Contrato de Mídia e Scanner (`MediaScanner`)
Na entidade `MediaItem`:
- Adicionar campos `source` (ex.: "Camera", "WhatsApp", "Screenshots", "Telegram", "Download", "Outros") e `originalRelativePath`.
- Adicionar método estático `detectSource(String path)` para categorização padronizada com base em padrões de caminho do sistema de arquivos Android/Linux/iOS.

### 3. Sincronização e Deduplicação (`ApiClient` / `WebDavSyncService`)
- Método `dedupFile({required String hash, required String storagePath, required String fileName, String? source, String? originalPath})` no `ApiClient`.
- Em `WebDavSyncService.syncPendingMedia(...)`:
  1. Para cada arquivo pendente, calcula `sha256`.
  2. Executa `checkHash(sha256)`.
  3. Se `exists == true`: chama `dedupFile(...)` sem transmitir o payload binário do arquivo, marca como sincronizado e avança.
  4. Se `exists == false`: envia via WebDAV PUT com os cabeçalhos `X-Mocambo-Source` e `X-Mocambo-Original-Path`.

### 4. Interface de Galeria (`GalleryPage`)
- Segmentação/Filtro por procedência no topo da galeria.
- Exibição do selo de procedência em cada miniatura de mídia.

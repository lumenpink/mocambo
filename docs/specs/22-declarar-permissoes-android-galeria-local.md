# Especificação Técnica — Ticket #22: Permissões de Armazenamento no Android e Integração com Galeria Local

## 1. Contexto & Objetivos
Até o momento, o app Mocambo não declarava as permissões de acesso a arquivos e mídia nos manifestos e a lista de itens na timeline era uma coleção mockada vazia em memória.

Os objetivos deste ticket são:
1. **Declaração de Permissões Android:** Inserir no `mobile/android/app/src/main/AndroidManifest.xml` as permissões canônicas de leitura de mídia e rede:
   - `android.permission.INTERNET`
   - `android.permission.READ_MEDIA_IMAGES`
   - `android.permission.READ_MEDIA_VIDEO`
   - `android.permission.READ_EXTERNAL_STORAGE` (com `maxSdkVersion="32"`)
   - `android.permission.ACCESS_MEDIA_LOCATION`
2. **Declaração no iOS:** Inserir chaves descritivas de uso de biblioteca de fotos no `Info.plist`.
3. **Serviço de Varredura Local (`MediaScanner`):** Implementar serviço de escaneamento de diretórios locais (DCIM, Pictures, Câmera ou pastas configuradas) que localiza arquivos de imagem/vídeo, calcula o hash SHA-256 e gera instâncias reais de `MediaItem`.
4. **Interface da Galeria:** Permitir ação de escanear/recarregar a galeria local com pré-visualização das fotos e exibição do estado de sincronização.

---

## 2. Critérios de Aceite & Verificação DD-TDD
- [ ] Teste automatizado em `tests/mobile_contract_test.rs` validando que:
  - `AndroidManifest.xml` declara `READ_MEDIA_IMAGES` e `INTERNET`.
  - `Info.plist` declara `NSPhotoLibraryUsageDescription`.
  - O serviço `MediaScanner` existe em `mobile/lib/services/media_scanner.dart` com suporte a indexação e hash.
- [ ] Renderização real de imagens locais na `GalleryPage`.
- [ ] Conformidade 100% no Quality Gate.

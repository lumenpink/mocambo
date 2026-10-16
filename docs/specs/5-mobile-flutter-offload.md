# Especificação Técnica: App Flutter com Galeria e Offload Inteligente de 15 Dias

- **Ticket**: [#5](https://codeberg.org/lumenpink/mocambo/issues/5)
- **Status**: Especificado / Fase 3
- **Data**: 2026-10-16
- **Autor**: Lumen Pink & Antigravity

---

## 1. Visão Geral e Propósito

Esta especificação define o **cliente móvel dedicado** do ecossistema **Mocambo**, desenvolvido em **Flutter** para Android e iOS. O aplicativo fornece uma interface fluida de galeria/timeline para fotos e vídeos, sincronização em segundo plano via WebDAV e uma política inovadora de **offload inteligente de 15 dias**.

O offload inteligente garante que mídias com mais de 15 dias só sejam expurgadas da memória local do smartphone após a **confirmação atômica e criptográfica** de que o arquivo já foi completamente sincronizado e persistido com integridade no backend Mocambo (`HEAD /api/files/check/{hash}` retornando `200 OK`).

---

## 2. Contratos e Arquitetura

### 2.1 Estrutura de Diretórios do Módulo Móvel (`mobile/`)

O código do aplicativo móvel reside no diretório `mobile/` na raiz do monorepo:

```
mobile/
├── pubspec.yaml
├── README.md
├── analysis_options.yaml
├── lib/
│   ├── main.dart
│   ├── config/
│   │   └── app_config.dart
│   ├── models/
│   │   ├── media_item.dart
│   │   └── search_result.dart
│   ├── services/
│   │   ├── api_client.dart
│   │   ├── offload_manager.dart
│   │   └── webdav_sync.dart
│   └── views/
│       ├── gallery_page.dart
│       ├── search_page.dart
│       └── settings_page.dart
└── test/
    ├── offload_manager_test.dart
    └── api_client_test.dart
```

### 2.2 Modelos de Dados

#### `MediaItem`
Representa um ativo de mídia no dispositivo:
- `id`: Identificador único local.
- `filePath`: Caminho no sistema de arquivos do dispositivo.
- `fileName`: Nome do arquivo.
- `fileSize`: Tamanho em bytes.
- `createdAt`: Timestamp de captura/criação da mídia.
- `sha256`: Hash criptográfico SHA-256 do arquivo.
- `syncStatus`: `pending`, `synced`, `offloaded`.

#### `SearchResult`
Representa o resultado de busca natural retornado pelo backend Mocambo:
- `id`: Identificador no catálogo do servidor.
- `storagePath`: Caminho relativo no storage do servidor.
- `fileName`: Nome do arquivo.
- `fileSize`: Tamanho em bytes.
- `snippet`: Trecho de correspondência ou score de similaridade visual.
- `matchType`: `text`, `semantic_image` ou `hybrid`.

### 2.3 Regras de Negócio do Offload Inteligente (`OffloadManager`)

O processo de offload opera sob três invariantes rígidos:

1. **Janela Temporal de 15 Dias**:
   - `idade = dataAtual - dataCriacao`
   - Se `idade < 15 dias`: a mídia é mantida intocada no armazenamento local (cache quente).
2. **Validação Criptográfica Pré-Expurgo**:
   - Para qualquer mídia com `idade >= 15 dias`, o worker calcula o SHA-256 do arquivo local.
   - Envia uma requisição HTTP `HEAD /api/files/check/{hash}` para o servidor Mocambo.
   - **Somente** se a resposta for HTTP `200 OK`, a mídia é elegível para expurgo.
3. **Exclusão Atômica e Preservação de Miniatura**:
   - O arquivo original de alta resolução é removido do armazenamento local do aparelho.
   - O status é atualizado para `offloaded`.
   - Se a requisição falhar (servidor inacessível, erro 404 de hash não encontrado, offline), a exclusão é **abortada imediatamente**, garantindo perda zero de dados.

### 2.4 Interface de Busca em Linguagem Natural

A tela `SearchPage` consome diretamente o endpoint `GET /api/search?q={termo}&limit=50`:
- Permite pesquisas por texto em documentos (FTS5) e por descrição visual em fotos (ex: *"praia"*, *"gato"*, *"recibo"* via CLIP).
- Exibe miniaturas em streaming ou via WebDAV.

---

## 3. Critérios de Aceite e Verificação

1. Estrutura completa do app Flutter implementada sob `mobile/`.
2. Modelos `MediaItem` e `SearchResult` serializáveis.
3. `ApiClient` com suporte a autenticação, checagem de hash `HEAD /api/files/check/{hash}` e busca `/api/search`.
4. `OffloadManager` com implementação das regras de retenção de 15 dias e validação de hash antes da exclusão.
5. Testes automatizados em Rust (`tests/mobile_contract_test.rs`) validando que o backend atende com fidelidade a todos os contratos consumidos pelo app móvel.
6. Conformidade Gosplan estrita: `cccp check --strict` com 100% de aprovação.

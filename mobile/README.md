# Mocambo Mobile

Cliente móvel soberano em Flutter para **Mocambo** (Android & iOS).

## Funcionalidades
- **Linha do Tempo Fluida**: Galeria de fotos e vídeos organizada cronologicamente.
- **Offload Inteligente de 15 Dias**: Libera espaço local com garantia criptográfica (cálculo de SHA-256 local e checagem de paridade com `HEAD /api/files/check/{hash}`).
- **Busca Semântica & FTS5**: Pesquisa em linguagem natural por texto em documentos e características visuais em imagens (CLIP).
- **Sincronização Segura**: Upload em segundo plano via WebDAV sem perda de dados.

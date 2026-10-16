# Especificação Técnica — Ticket #8: Web UI Estática Embarcada

## 1. Contexto & Objetivos
Atualmente, o Mocambo expõe suas operações através de protocolo WebDAV, API REST de busca (`/api/search`) e compartilhamento Fuxico (`/api/share`). Para permitir que qualquer usuário visualize seus arquivos, fotos e vídeos através de um navegador comum sem necessidade de clientes WebDAV dedicados, o servidor deve embarcar uma **Web UI Estática Minimalista**.

### Diretrizes de Design
1. **Zero Runtime Dependencies**: HTML5, CSS3 e Vanilla JavaScript embarcados diretamente no binário Rust via `include_str!`, mantendo a imagem Docker em ~8 MB.
2. **Estética Brutalista Soberana**: Dark mode nativo com alto contraste (`#0d0f12`, verde esmeralda `#00e699`, âmbar `#ffd700`), tipografia monospaçada e responsividade móvel.
3. **Explorador & Galeria**: Navegação em árvore de pastas, visualização em lista ou grade (grid) com thumbnails e lightbox modal.
4. **Busca Integrada**: Campo de busca instantâneo conectado a `/api/search` combinando busca textual FTS5 e busca semântica CLIP.

---

## 2. Arquitetura de Rotas e Endpoints

```mermaid
flowchart TD
    User["Navegador Web (Desktop / Mobile)"] -->|GET / ou /ui| WebUI["Handler Web UI (HTML/CSS/JS embutidos)"]
    WebUI -->|GET /api/files?path=/| FileAPI["Listagem de Diretórios POSIX"]
    WebUI -->|GET /api/search?q=...| SearchAPI["Busca Híbrida FTS5 + Vetores"]
    WebUI -->|GET /raw/{caminho}| RawFile["Streaming de Arquivos & Mídias"]
```

### 2.1. Rotas HTTP
- `GET /` ou `GET /ui`: Retorna o documento HTML da interface web com headers de segurança (`Content-Type: text/html; charset=utf-8`, CSP, X-Frame-Options).
- `GET /api/files?path={dir}`: Retorna array JSON de itens no diretório solicitado:
  ```json
  [
    {
      "name": "fotos",
      "path": "/fotos",
      "is_dir": true,
      "size": 0,
      "modified": 1729000000,
      "mime_type": "inode/directory"
    },
    {
      "name": "documento.pdf",
      "path": "/fotos/documento.pdf",
      "is_dir": false,
      "size": 1048576,
      "modified": 1729000000,
      "mime_type": "application/pdf"
    }
  ]
  ```
- `GET /raw/*path`: Serve o arquivo bruto armazenado sob `storage_path`, com suporte a MIME types corretos e tratamento de HTTP Range para reprodução de áudio e vídeo no navegador.

---

## 3. Segurança & Resiliência
- **Prevenção de Path Traversal**: Validação rigorosa de qualquer caminho requisitado via `/api/files` ou `/raw/`. Tentativas contendo `..` ou resolvendo fora do `storage_path` resultam em `403 Forbidden` ou `400 Bad Request`.
- **Sanitização de XSS**: Nomes de arquivos e snippets de texto retornados na busca devem ser escapados antes da renderização no DOM (`escapeHtml`).
- **Degradação Graciosa**: Se nenhum arquivo existir, exibe mensagem clara com atalho para envio via WebDAV.

---

## 4. Critérios de Aceite & DD-TDD
- [ ] Teste de integração (`tests/web_ui_test.rs`) validando:
  - `GET /` e `GET /ui` retornando 200 OK com `text/html` contendo componentes da Web UI.
  - `GET /api/files?path=/` retornando lista formatada de arquivos e diretórios em JSON.
  - `GET /raw/{file}` servindo o arquivo armazenado com MIME type apropriado.
  - Bloqueio de Directory Traversal em caminhos maliciosos (`/../`, `../etc/passwd`).
  - Renderização da barra de busca e integração com `/api/search`.
- [ ] Implementação do módulo `web_ui` embutido no servidor Axum do Mocambo.
- [ ] Suíte completa de testes passando sem alertas do Clippy ou violações de formatação.

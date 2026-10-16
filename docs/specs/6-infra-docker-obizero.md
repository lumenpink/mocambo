# Especificação Técnica: Conteinerização Estática e Stack no Obizero

- **Ticket**: [#6](https://codeberg.org/lumenpink/mocambo/issues/6)
- **Status**: Especificado / Fase 3
- **Data**: 2026-10-16
- **Autor**: Lumen Pink & Antigravity

---

## 1. Visão Geral e Propósito

Esta especificação define a arquitetura de **empacotamento estático em contêiner** e a **composição da stack de produção do Mocambo** no nó homelab `obizero` (integrado ao ecossistema soberano Baquara).

O objetivo é disponibilizar o backend do Mocambo em uma imagem Docker minimalista, auto-contida, estática (`x86_64-unknown-linux-musl`), com tamanho inferior a 25 MB, roteada através do proxy reverso Caddy sob o domínio `vault.{$DOMAIN:lohn.in}` com proteção granular da VPN Tailscale e rota pública de compartilhamento.

---

## 2. Contratos e Arquitetura

### 2.1 Dockerfile Multi-Stage Musl Estático

O `Dockerfile` é estruturado em dois estágios:

1. **Estágio de Build (`builder`)**:
   - Imagem base: `rust:alpine` com dependências de compilação essenciais (`musl-dev`, `build-base`, `perl`, `make`).
   - Compilação estática com target `x86_64-unknown-linux-musl` em perfil `release` (com LTO e stripping de símbolos).
2. **Estágio Final de Runtime (`runtime`)**:
   - Imagem base: `alpine:3.20` ou `scratch` com certificados SSL/TLS da CA atualizados (`ca-certificates`) e fuso horário (`tzdata`).
   - Usuário sem privilégios (`appuser:10001`).
   - Volumes de dados: `/data/storage` e `/data/db`.
   - Porta exposta: `8080`.
   - Healthcheck nativo consultando `GET /health`.

### 2.2 Composição da Stack no Obizero (`obizero/stacks/mocambo/compose.yml`)

No repositório `baquara-infra` (`obizero/stacks/mocambo/`):

```yaml
services:
  mocambo:
    image: mocambo:latest
    build:
      context: ../../../../../mocambo
      dockerfile: Dockerfile
    container_name: mocambo
    restart: unless-stopped
    user: "1000:1000"
    environment:
      - PORT=8080
      - HOST=0.0.0.0
      - STORAGE_DIR=/data/storage
      - DB_PATH=/data/mocambo.db
      - FUXICO_BASE_URL=${FUXICO_BASE_URL:-https://2lp.in}
      - FUXICO_API_TOKEN=${FUXICO_API_TOKEN:-}
      - MOCAMBO_INTERNAL_URL=${MOCAMBO_INTERNAL_URL:-http://mocambo:8080}
    volumes:
      - /srv/mocambo/storage:/data/storage
      - /srv/mocambo/data:/data
    networks:
      - infra_net
    healthcheck:
      test: ["CMD-SHELL", "wget -q --spider http://127.0.0.1:8080/health || exit 1"]
      interval: 30s
      timeout: 5s
      retries: 3

networks:
  infra_net:
    external: true
```

### 2.3 Roteamento no Proxy Reverso Caddy (`Caddyfile`)

No Caddyfile do `obizero` (`obizero/stacks/infra/static/caddy/Caddyfile`), adiciona-se o bloco para `vault.{$DOMAIN:lohn.in}`:

```caddy
# Mocambo Sovereign Storage & WebDAV
vault.{$DOMAIN:lohn.in} {
	# Rota pública para links de download/compartilhamento direto
	@public_share path /s/*
	handle @public_share {
		reverse_proxy mocambo:8080
	}

	# Todas as demais rotas (WebDAV /dav/*, APIs /api/*, busca) protegidas por VPN
	handle {
		import vpn_only
		reverse_proxy mocambo:8080
	}
}
```

---

## 3. Critérios de Aceite e Verificação

1. `Dockerfile` presente na raiz do repositório `mocambo` compilando binário estático musl e gerando imagem mínima.
2. Composição `compose.yml` e `.env.example` definidos no repositório `mocambo` (`deploy/compose.yml`) e integrados em `baquara-infra/obizero/stacks/mocambo/`.
3. Bloco `vault.{$DOMAIN:lohn.in}` configurado no `Caddyfile` com `import vpn_only` e exceção pública para `/s/*`.
4. Teste automatizado de infraestrutura (`tests/infra_docker_obizero_test.rs`) validando as regras de Dockerfile, Compose e diretivas do Caddy.
5. Conformidade Gosplan estrita: `cccp check --strict` com 100% de aprovação.

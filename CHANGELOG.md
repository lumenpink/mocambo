# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Bug Fixes
- **dist:** assinar apk soberanamente com in.twolp e injetar versao canonica (closes #10)

## [0.2.0] - 2026-10-16

### Features
- **s3:** implementar camada de compatibilidade s3 minima (#9)
- **ui:** implementar web ui estatica e streaming de arquivos (#8)
- **dist:** esteira cicd para banca e apk na patilera (#7)

### Bug Fixes
- **dist:** alinhar categoria apps com contrato da banca (#7)

### Documentation
- **changelog:** registrar compatibilidade s3 (closes #9)
- **s3:** especificacao da camada de compatibilidade s3 (#9)
- **changelog:** registrar web ui estatica (closes #8)
- **ui:** especificacao da web ui estatica embarcada (#8)
- **changelog:** registrar esteira banca e patilera (closes #7)
- **dist:** especificacao distribuicao banca e patilera (#7)

### Testing
- **s3:** testes de mutacao para prefixos s3 e delecao (#9)
- **s3:** teste red para operacoes s3 put get list delete (#9)
- **ui:** testes de mutacao para csp e bloqueio de diretorios (#8)
- **ui:** teste red para web ui e listagem de arquivos (#8)
- **dist:** testes de mutacao para icones png e flags cli (#7)
- **dist:** teste red para distribuicao banca e patilera (#7)

### Maintenance
- **git:** ignorar diretorio de build mobile e apks (#9)
- **version:** sincronizar VERSION para 0.2.0-dev.22 (#9)
- **s3:** formatacao de codigo com cargo fmt (#9)
- **version:** sincronizar VERSION para 0.2.0-dev.15+20261016.e900e5c
- **ui:** formatacao e clippy na web ui (#8)
- **version:** sincronizar VERSION para 0.2.0-dev.8+20261016.6f18aa6
- **dist:** formatacao de testes com cargo fmt (#7)

## [0.1.0] - 2026-10-16

### Features
- **mobile:** app flutter com galeria e offload 15 dias (#5)
- **infra:** dockerfile estatico musl e manifests de deploy (#6)
- **ai:** busca semantica em imagens com vetores e rrf (#4)
- **ai:** implementar pipeline extrator e busca fts5 (#3)
- **share:** integracao com fuxico e streaming interno (ref #2)
- **core:** storage posix webdav e checagem de hash (#1)

### Documentation
- **changelog:** registrar app flutter e offload (closes #5)
- **mobile:** especificacao do app flutter e offload 15 dias (#5)
- **changelog:** registrar conteinerizacao obizero (closes #6)
- **infra:** especificacao da conteinerizacao e stack obizero (#6)
- **changelog:** registrar busca semantica em imagens (closes #4)
- **ai:** especificacao da busca semantica em imagens (#4)
- **changelog:** registrar fts5 textual search (closes #3)
- **ai:** especificacao tecnica do pipeline de extracao e fts5 (#3)
- **changelog:** registrar integracao fuxico (closes #2)
- **share:** especificacao tecnica da integracao fuxico (#2)
- **spec:** contratos de storage posix webdav e hash (#1)

### Testing
- **mobile:** teste red para estrutura flutter e contrato api (#5)
- **infra:** teste red para dockerfile e caddyfile obizero (#6)
- **ai:** teste red para busca semantica de imagens (#4)
- **ai:** teste red para pipeline assincrono e busca fts5 (#3)
- **share:** revogacao automatica por delecao webdav (#2)
- **share:** ciclo de vida de share fuxico e streaming interno (#2)
- **core:** mutacao para delete overwrite e mkcol (#1)
- **core:** teste integracao webdav e storage posix (#1)

### Maintenance
- **version:** sincronizar VERSION para 0.1.0-dev.33 (#5)
- **version:** sincronizar VERSION para 0.1.0-dev.28 (#6)
- **version:** sincronizar VERSION para 0.1.0-dev.23 (#4)
- **version:** sincronizar VERSION para 0.1.0-dev.18 (#3)
- **version:** sincronizar versao para fechamento (closes #2)
- **version:** sincronizar VERSION para 0.1.0-dev.7+20261016.76049a9
- **core:** formatacao e registro de entrega (closes #1)
- inicializar repositorio mocambo sob padrao gost (#1)

## [Unreleased]

### Added
  - (s3) camada minima de compatibilidade com a api s3 da amazon (closes #9)
  - (ui) interface web estatica minimalista embarcada no binario (closes #8)
  - (dist) esteira ci/cd para banca e publicacao de apk na patilera (closes #7)
  - (mobile) app flutter com galeria e offload inteligente de 15 dias (closes #5)
  - (infra) conteinerizacao estatica e stack no obizero com caddy (closes #6)
  - (ai) busca semantica em imagens com vetores e fusao rrf (closes #4)
  - (ai) pipeline assincrono de extracao textual e busca fts5 (closes #3)
  - (share) integracao de compartilhamento com fuxico e streaming interno (closes #2)
  - (core) implementar motor de storage posix, webdav e checagem de hash sha256 (closes #1)
  - Inicialização do repositório em conformidade com o padrão GOST.

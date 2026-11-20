# Especificação Técnica — Ticket #28: Pipeline de Empacotamento Android e Publicação Contínua na Pratilera

## 1. Contexto e Motivação
A publicação do aplicativo móvel Mocambo (`in.twolp.mocambo`) na loja descentralizada Pratilera (`https://banca.2lp.in/pratilera`) vinha operando com falhas silenciosas na esteira de CI/CD:
1. O empacotamento do APK estava embutido no mesmo job de compilação da CLI Rust (`gost-ci-cross`), onde não existe Flutter nem SDK do Android.
2. Como resultado, o script `scripts/package-apk.sh` caía invariavelmente no fallback de contingência (`IS_REAL_BUILD=false`), abortando o upload para o Cloudflare R2 (`twolp-bucket/mocambo.apk`).
3. O catálogo da Pratilera ao vivo consumia um APK jurássico pré-existente no R2 com `versionName='0.1.0'` e `versionCode='1'`, ignorando a evolução do SemVer canônico do CCCP (`0.3.0-dev.NN...`).
4. O `pubspec.yaml` do módulo Flutter mantinha a versão estática `0.1.0+1`, e a chamada ao `flutter build apk` não injetava `--build-name` nem `--build-number`.

Esta especificação formaliza o pipeline hermético, permanente e auditável de compilação, assinatura, publicação no Cloudflare R2 e notificação à Banca.

---

## 2. Árvore de Decisões e Proteção à Natasha
Para que a Natasha compreenda os motivos de cada escolha e não reintroduza dívidas técnicas:

### Decisão 1: Desacoplar o build do APK em um job dedicado (`deploy-apk`)
- **Alternativa Descartada A:** Instalar Flutter dinamicamente no container `gost-ci-cross`.
  - *Motivo do Descarte:* Quebra a hermeticidade, infla o tempo de CI com downloads repetitivos de gigabytes e viola o isolamento de ambientes da norma GOST.
- **Alternativa Descartada B:** Manter build de APK apenas manual em máquina de desenvolvimento.
  - *Motivo do Descarte:* Quebra o princípio da entrega contínua autônoma; a cada release ou push na `main`, a loja ficaria defasada caso a pessoa mantenedora esquecesse do upload manual.
- **Escolha Adotada:** Criar o job `deploy-apk` no `.forgejo/workflows/ci.yml` rodando exclusivamente no container oficial `gambi.2lp.in/lumen/gost-ci-flutter:latest`, onde Android SDK 34, OpenJDK 17 e Flutter SDK estável já se encontram pré-aquecidos.

### Decisão 2: Derivação canônica do `versionCode` no Android
- **Regra Android/F-Droid:** O Android OS e os clientes da Pratilera (Droid-ify / Neo Store / F-Droid) recusam ou ignoram atualizações de aplicativos se o `versionCode` não for um número inteiro estritamente crescente em relação à versão instalada.
- **Alternativa Descartada:** Usar timestamp Unix (`date +%s`).
  - *Motivo do Descarte:* Gera números gigantes que podem estourar o limite de 32 bits do Android `versionCode` (máximo `2100000000`) e perde a semântica de commits da árvore Git.
- **Escolha Adotada:** Extrair o sufixo `dev.NN` da versão canônica SemVer do CCCP (exemplo: `0.3.0-dev.45+...` $\rightarrow$ `versionCode = 45`). Em tags estáveis sem `dev.NN`, somar uma base determinística com o número total de commits do histórico git (`git rev-list --count HEAD`), garantindo monotonicidade estrita.

### Decisão 3: Encadeamento de notificação para a Pratilera (`trigger-pratilera`)
- **Problema anterior:** A Banca era notificada antes do upload do APK ou mesmo quando o APK falhava. A Banca acordava, baixava o APK antigo do R2 e perpetuava a versão defasada.
- **Escolha Adotada:** O passo de notificação de webhook (`Trigger Pratilera Store Update`) torna-se dependente estrito de `deploy-apk` (`needs: [deploy-apk]`), executando o dispatch HTTP somente após o arquivo `twolp-bucket/mocambo.apk` ser confirmado com sucesso no Cloudflare R2.

---

## 3. Contratos de Arquitetura e Interfaces

### 3.1. Script `scripts/package-apk.sh`
- **Extração da Versão Canônica:**
  - `APP_VERSION`: lida de `VERSION` ou `Cargo.toml`.
  - `BUILD_NUMBER`: extraído de `dev.([0-9]+)` em `APP_VERSION`, ou `git rev-list --count HEAD`.
- **Injeção no Flutter:**
  - Atualização determinística em `mobile/pubspec.yaml` (`version: ${APP_VERSION_BASE}+${BUILD_NUMBER}`).
  - Chamada `flutter build apk --release --build-name="$APP_VERSION" --build-number="$BUILD_NUMBER"`.
- **Assinatura:**
  - Checagem da keystore oficial `in_twolp_release.keystore` e execução de `scripts/sign-release.sh`.
- **Validação de Tamanho:**
  - Binário final deve possuir tamanho mínimo de 10 MB (`10485760` bytes) para ser classificado como build real (`IS_REAL_BUILD=true`). Pacotes de contingência continuam permitidos em runners de teste sem Flutter, mas nunca são marcados como reais.

### 3.2. Workflow `.forgejo/workflows/ci.yml`
- **Job `deploy-apk`:**
  - `needs: [test]`
  - `runs-on: ubuntu-latest`
  - `container: gambi.2lp.in/lumen/gost-ci-flutter:latest`
  - `if: github.ref == 'refs/heads/main' || startsWith(github.ref, 'refs/tags/v')`
  - Passos:
    1. Checkout do repositório.
    2. Execução de `scripts/package-apk.sh`.
    3. Validação do APK gerado (`stat` >= 10MB).
    4. Upload de `site/mocambo.apk` para `twolp-bucket/mocambo.apk` via Wrangler R2 com 4 tentativas de retry exponencial.
- **Job `trigger-pratilera`:**
  - `needs: [deploy-apk]`
  - `runs-on: ubuntu-latest`
  - Dispara o webhook `workflows/ci.yml/dispatches` na Banca.

---

## 4. Testes de Contrato (Fase 4 - Red)
O teste `tests/distribution_contract_test.rs` deve validar:
1. Existência e integridade do job `deploy-apk` no workflow `ci.yml`.
2. Uso da imagem `gambi.2lp.in/lumen/gost-ci-flutter:latest`.
3. Injeção de `--build-name` e `--build-number` em `scripts/package-apk.sh`.
4. Upload para `twolp-bucket/mocambo.apk`.
5. Dependência explícita `needs: [deploy-apk]` no disparo da Banca.

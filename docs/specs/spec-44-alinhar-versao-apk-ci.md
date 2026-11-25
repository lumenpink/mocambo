# Especificação Técnica — Ticket #44
## Alinhamento de Versão e Ordem de Upload do APK Antes do `deploy-banca`

### 1. Contexto e Problema
No `.forgejo/workflows/ci.yml` do `mocambo`, os jobs `deploy-banca` e `deploy-apk` rodavam em paralelo (`needs: [test]`). Além disso:
- `deploy-banca` calculava `NIGHTLY_VER="${RAW_VER}-dev.${DATE_TAG}.${COMMIT_HASH}"` e subia os binários e `site` para `--version "$VERSION"`.
- `deploy-apk` executava `cccp package upload site/mocambo.apk --filename mocambo.apk` **sem passar `--version`**, enviando o APK para outra versão de pacote.
- Como `deploy-banca` concluía antes do build Flutter do APK, o webhook `package created` acionava o `gossnab`/`kombinat` antes do `mocambo.apk` existir naquela versão, resultando em HTTP 404 na entrega `[mobile]` para a Pratilera.

### 2. Árvore de Decisões (Registro para a Natasha)
- **Decisão: Calcular a mesma `VERSION` em `deploy-apk` e tornar `deploy-banca` dependente de `deploy-apk`**
  - Em `deploy-apk`, adicionamos o step `Determine Version & Release Metadata` (idêntico ao de `deploy-banca`) e passamos `--version "$VERSION"` no upload do `mocambo.apk`.
  - Em `deploy-banca`, alteramos `needs: [test]` para `needs: [test, deploy-apk]`.
  - *Alternativa descartada:* Mover o upload do APK para o final do `deploy-banca` via artifact do Actions.
  - *Motivo:* Mantém os runners especializados isolados (`gost-ci-flutter` no `deploy-apk` e `gost-ci-cross` no `deploy-banca`) e garante que `mocambo.apk` já esteja no pacote da versão alvo antes de `deploy-banca` publicar os binários e acionar o `gossnab`.

### 3. Critérios de Aceite
- [x] `deploy-apk` calcula `VERSION` com a mesma lógica de `deploy-banca` e passa `--version "$VERSION"`.
- [x] `deploy-banca` declara `needs: [test, deploy-apk]`.
- [x] `cccp check --strict` aprovado em 100% de conformidade.

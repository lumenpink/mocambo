# Especificação Técnica #34: Assinatura Soberana de APK com Keystore Oficial in.twolp

## 1. Contexto e Diagnóstico

Na distribuição do aplicativo Mocambo via Pratilera (catálogo F-Droid servido em `https://banca.2lp.in/pratilera`), o cliente Droid-ify reportou erro de divergência de assinatura do pacote `in.twolp.mocambo`.

### Análise da Causa Raiz
1. O certificado registrado na Pratilera (`in_twolp_release.keystore`) possui o Subject DN `CN=in.twolp, OU=Sovereign Software, O=Twolp, L=Florianopolis, ST=SC, C=BR` com digest SHA-256 `7B:EE:C8:B0:A6:C7:94:6B:04:D5:38:54:3D:49:5A:82:10:AD:6C:FC:18:59:F4:2E:71:80:BC:FB:D3:8F:E5:82`.
2. O arquivo `mobile/android/app/build.gradle.kts` utilizava a configuração provisória do template Flutter: `signingConfig = signingConfigs.getByName("debug")`.
3. O script `scripts/package-apk.sh` tentava invocar `scripts/sign-release.sh` fora do container, no host do runner (`gost-ci-cross`), onde o `apksigner` não existe. O script emitia aviso e mantinha o APK assinado com a chave debug do Android (`CN=Android Debug`), provocando o bloqueio pelo Droid-ify.

## 2. Decisão Arquitetural e Porquê para a Natasha

Para Natasha: como manter a assinatura soberana sem quebrar builds locais de desenvolvedores sem a chave?
1. **Padrão Canônico Adotado:**
   - Em `mobile/android/app/build.gradle.kts`, configurar `signingConfigs.create("release")` que verifica dinamicamente a presença de:
     - `~/.android/in_twolp_release.properties`
     - `~/.android/in_twolp_release.keystore`
   - Se ambos existirem, o Gradle carrega as propriedades (`storePassword`, `keyAlias`, `keyPassword`) e assina o release com a chave oficial `in.twolp`.
   - Se não existirem, faz fallback gracioso para `debug.keystore`, permitindo que desenvolvedores compilem localmente sem erro.
2. **Validação de Guarda Estrita em CI (Gate de Segurança):**
   - Dentro de `scripts/package-apk.sh`, após a geração do APK, o script deve invocar o `apksigner` da imagem hermética `gost-ci-flutter` (`/opt/android-sdk/build-tools/34.0.0/apksigner`) e validar que o certificado contém `CN=in.twolp`.
   - Se o APK não estiver assinado pela chave soberana, o script falha com exit code 1 antes de qualquer upload ao Cloudflare R2 ou dispatch para a Banca.

## 3. Contrato de Teste (Fase 4 - Red Test)

Em `tests/distribution_contract_test.rs`:
- Validar que `mobile/android/app/build.gradle.kts` configura `in_twolp_release` em `signingConfigs`.
- Validar que `scripts/package-apk.sh` realiza checagem de integridade de certificado `CN=in.twolp`.

## 4. Plano de Implementação

1. **Fase 4 (Red Test):** Criar testes contratuais em `tests/distribution_contract_test.rs`.
2. **Fase 5 (Green Implement):** 
   - Atualizar `mobile/android/app/build.gradle.kts`.
   - Atualizar `scripts/package-apk.sh` com a validação soberana.
3. **Fase 6 (Refactor):** Assegurar imports limpos e mensagens de erro legíveis.
4. **Fase 7 (Mutation Test):** Validar comportamento com propriedades ausentes.
5. **Fase 8 (Quality Gate):** Validar `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` e `cargo test --test distribution_contract_test`.
6. **Fase 9 (Finish & Close):** Finalização com `cccp close`.

## 5. Validação e Encerramento

- **Testes de Contrato:** 10/10 testes aprovados em `tests/distribution_contract_test.rs`.
- **Quality Gates:** `cargo fmt --check` e `cargo clippy --all-targets -- -D warnings` sem pendências.
- **Assinatura Soberana:** Configurado `signingConfigs.release` e gate estrito de certificado `CN=in.twolp`.


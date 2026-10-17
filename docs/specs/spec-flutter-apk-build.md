# Especificação Técnica: Compilação Hermética do Flutter APK e Publicação no `twolp-bucket`

## 1. Contexto e Motivação
O aplicativo mobile oficial do Mocambo (`in.twolp.mocambo`) possui interface e serviços implementados em Flutter/Dart em `mobile/lib/`. Anteriormente, por ausência da toolchain Flutter no runner local, um APK stub binário mínimo era gerado.
Em conformidade com a norma GOST, a compilação passa a ser realizada hermeticamente utilizando a sub-imagem atômica `gost-ci-flutter`, garantindo geração do binário real multi-arch (`arm64-v8a` + `x86_64`) sem `strip` destrutivo.

## 2. Requisitos de Build & Assinatura
1. **Compilação Flutter**:
   - `flutter build apk --release` (multi-arch `arm64-v8a` e `x86_64`).
   - Binário final produzido em `site/mocambo.apk`.
   - Tamanho esperado: > 15 MiB (contendo motor Flutter e bibliotecas nativas).
2. **Assinatura Soberana**:
   - Assinatura com a keystore oficial `~/.android/in_twolp_release.keystore` (SHA-256 digest `7beec8b0a6c7946b04d538543d495a8210ad6cfc1859f42e7180bcfbd38fe582`).
3. **Distribuição via Cloudflare R2 (`twolp-bucket`)**:
   - Upload para `twolp-bucket/mocambo.apk` via `wrangler r2 object put`.
   - Distribuição direta aos clientes sob `https://bucket.2lp.in/mocambo.apk`.
4. **Sincronização Patilera**:
   - Sincronização dos metadados e do APK no repositório da Banca com regeneração do catálogo F-Droid (`index-v1.json`).

# Spec #45 — Restaurar `app_id` canônico `in.twolp.mocambo` em `.gost/delivery.toml`

- **Ticket:** `#45`
- **Escopo:** `.gost/delivery.toml`

## Contexto e Árvore de Decisões
Na migração do CD declarativo (ph9, commit `9d25943`), `.gost/delivery.toml` foi criado com `app_id = "org.pink.mocambo"`, divergindo do `applicationId = "in.twolp.mocambo"` definido em `mobile/android/app/build.gradle.kts` e `scripts/package-apk.sh`. Isso fez o `kombinat` criar uma entrada duplicada na Pratilera.
- **Escolha adotada:** alinhar `.gost/delivery.toml` para `in.twolp.mocambo`, mantendo compatibilidade com instalações Android existentes.
- **Alternativa descartada:** alterar o `applicationId` do Android para `org.pink.mocambo`, o que quebraria assinaturas, atualizações e dados locais dos dispositivos.

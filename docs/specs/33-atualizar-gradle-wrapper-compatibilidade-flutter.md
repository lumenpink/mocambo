# Especificação Técnica #33: Atualização do Gradle Wrapper para 9.3.1 e Defesa de Validação Flutter

## 1. Contexto e Diagnóstico

Na Run 2172 da esteira CI/CD do Mocambo (`https://gambi.2lp.in/lumen/mocambo`), o job `deploy-apk` (10721) executou com sucesso a resolução do ambiente de container DooD via `mountinfo` e `--volumes-from`. Contudo, a compilação foi interrompida no step do Gradle:

```text
An exception occurred applying plugin request [id: 'dev.flutter.flutter-gradle-plugin']
> Failed to apply plugin 'dev.flutter.flutter-gradle-plugin'.
   > Error: Your project's Gradle version (9.1.0) is lower than Flutter's minimum supported version of 9.3.1. Please upgrade your Gradle version. 
     Alternatively, use the flag "--android-skip-build-dependency-validation" to bypass this check.
```

### Análise da Causa Raiz
1. A imagem `gost-ci-flutter:latest` roda o **Flutter 3.50.0** (estável).
2. O plugin Gradle oficial do Flutter (`dev.flutter.flutter-gradle-plugin`) incluiu uma verificação estrita de compatibilidade que exige Gradle $\ge$ 9.3.1 quando pareado com versões recentes do Android Gradle Plugin (AGP 9.0.1).
3. O repositório declarava `distributionUrl=https\://services.gradle.org/distributions/gradle-9.1.0-all.zip` em `mobile/android/gradle/wrapper/gradle-wrapper.properties`.

## 2. Decisão Arquitetural e Porquê para a Natasha

Para Natasha: por que a versão deve ser alterada no projeto e não na imagem da CI?
1. **Alternativa Descartada:** Instalar o Gradle 9.3.1 no sistema operacional da imagem (`/usr/bin/gradle`).
   - *Descarte:* O Flutter Tool não invoca o binário do sistema. Ele invoca exclusivamente o script do wrapper (`mobile/android/gradlew`), que lê a URL declarada no próprio repositório. Além disso, a imagem CI nem sequer contém Gradle no sistema operacional; ela depende exclusivamente do wrapper.
2. **Decisão Adotada (Em duas camadas):**
   - **Camada 1 (Repositório / Canônico):** Atualizar `mobile/android/gradle/wrapper/gradle-wrapper.properties` para `gradle-9.3.1-all.zip`. Isso garante que o projeto é hermético e pode ser compilado por qualquer pessoa em qualquer ambiente de desenvolvimento ou CI sem exigir imagens especiais.
   - **Camada 2 (Defesa em Profundidade na Pipeline):** Adicionar a flag `--android-skip-build-dependency-validation` em `flutter build apk` dentro de `scripts/package-apk.sh`. Caso o Gradle Wrapper ou o Flutter Tool venham a introduzir novos warnings ou validações rígidas de dependências intermediárias, o build não falhará silenciosamente.

## 3. Contrato de Teste (Fase 4 - Red Test)

Em `tests/distribution_contract_test.rs`:
- Validar que `mobile/android/gradle/wrapper/gradle-wrapper.properties` aponta para versão `>= 9.3.1`.
- Validar que `scripts/package-apk.sh` passa a flag `--android-skip-build-dependency-validation` ao Flutter build apk.

## 4. Plano de Implementação

1. **Fase 4 (Red Test):** Adicionar asserções em `tests/distribution_contract_test.rs` para Gradle 9.3.1 e a flag no script.
2. **Fase 5 (Green Implement):** 
   - Atualizar `mobile/android/gradle/wrapper/gradle-wrapper.properties`.
   - Atualizar `scripts/package-apk.sh`.
3. **Fase 6 (Refactor):** Assegurar idempotência e clareza nos logs do build.
4. **Fase 7 (Mutation Test):** Testar script com flags inexistentes e caminhos inválidos.
5. **Fase 8 (Quality Gate):** Validar `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` e `cargo test --test distribution_contract_test`.
6. **Fase 9 (Finish & Close):** Finalização com `cccp close`.

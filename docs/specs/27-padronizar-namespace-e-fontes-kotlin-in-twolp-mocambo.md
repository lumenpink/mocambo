# Especificação Técnica - Ticket #27
## Alinhar Namespace e Fontes Kotlin para in.twolp.mocambo

**ID do Ticket:** #27  
**Norma:** GOST 1.5.0 (DD-TDD)  
**Status:** Concluído (Fase 9)  
**Responsável:** Lumen Pink / Antigravity  

---

### 1. Objetivo e Motivação

Padronizar a estrutura do pacote Android do aplicativo mobile do Mocambo (`mobile/android`), alinhando o `namespace` Gradle e os fontes Kotlin com o identificador canônico de domínio reverso `in.twolp.mocambo`, já consolidado nos aplicativos irmãos `lauda` e `lacraia`.

Historicamente, o `applicationId` já estava definido como `in.twolp.mocambo` no `build.gradle.kts`, mas o `namespace` Gradle e o diretório Kotlin haviam sido invertidos para `twolp.in.mocambo` para evitar a colisão com a palavra reservada `in` da linguagem Kotlin. Esta divergência gera inconsistência arquitetural e quebra as verificações centralizadas do ecossistema autônomo.

---

### 2. Árvore de Decisão Técnica (Para a Natasha)

- **Decisão 1: Unificação com `in.twolp.mocambo` vs Manter Invertido `twolp.in.mocambo`:**
  - *Alternativa descartada:* Manter `twolp.in.mocambo` como namespace interno.
    - *Motivo do descarte:* Quebra a convenção de FQDN reverso padrão do ecossistema (`2lp.in` -> `in.twolp.*`) e introduz assimetria com `lauda` (`in.twolp.lauda`) e `lacraia` (`in.twolp.lacraia`). Scripts de validação e CI precisam tratar exceções desnecessárias.
  - *Escolha adotada:* **Adotar `in.twolp.mocambo` de ponta a ponta**.
    - No Gradle (`mobile/android/app/build.gradle.kts`): `namespace = "in.twolp.mocambo"`.
    - No código Kotlin (`MainActivity.kt`): ``package `in`.twolp.mocambo``, usando a sintaxe canônica com delimitadores de crase do Kotlin para a palavra reservada `in`.
    - No diretório de arquivos: mover de `twolp/in/mocambo/` para `in/twolp/mocambo/`.

- **Decisão 2: Preservação de Compatibilidade do `applicationId`:**
  - *Escolha adotada:* O `applicationId = "in.twolp.mocambo"` já era o identificador oficial do app. Sua manutenção estrita garante que nenhum usuário sofra perda de dados locais, sessão, cache ou identidade no dispositivo.

---

### 3. Arquivos Afetados

1. `mobile/android/app/build.gradle.kts`: alterar `namespace = "twolp.in.mocambo"` para `namespace = "in.twolp.mocambo"`.
2. Mover: `mobile/android/app/src/main/kotlin/twolp/in/mocambo/MainActivity.kt` para `mobile/android/app/src/main/kotlin/in/twolp/mocambo/MainActivity.kt`.
3. `MainActivity.kt`: alterar a declaração de pacote para ``package `in`.twolp.mocambo``.
4. Teste automatizado de contrato: adicionar verificação de conformidade do namespace e pacote Kotlin em testes.

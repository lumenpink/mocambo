# Especificação Técnica — Ticket #19: Padronizar Identidade Visual para Mocambo em Todas as Plataformas

## 1. Contexto & Objetivos
O aplicativo multiplataforma do Mocambo (construído em Flutter) apresentava sufixos técnicos (`mocambo_mobile` e `mocambo-mobile`) nos rótulos de sistema operacional, títulos de janela e manifestos de inicialização.

O objetivo deste ticket é garantir que a identificação visível para a usuária final seja estritamente **Mocambo** em todas as plataformas-alvo (Android, Linux, Windows, macOS, Web e camada Flutter), preservando identificadores canônicos internos de pacote (`applicationId` e namespaces técnicos).

---

## 2. Escopo de Alterações Declarativas

| Plataforma / Camada | Arquivo | Rótulo Anterior | Rótulo Canônico Novo |
| :--- | :--- | :--- | :--- |
| **Android** | `mobile/android/app/src/main/AndroidManifest.xml` | `android:label="mocambo_mobile"` | `android:label="Mocambo"` |
| **Flutter App** | `mobile/lib/main.dart` | `title: 'Mocambo Mobile'` | `title: 'Mocambo'` |
| **Linux (GTK)** | `mobile/linux/runner/my_application.cc` | `"mocambo_mobile"` | `"Mocambo"` |
| **Windows** | `mobile/windows/runner/main.cpp` e `Runner.rc` | `mocambo_mobile` | `Mocambo` |
| **macOS** | `mobile/macos/Runner/Configs/AppInfo.xcconfig` | `PRODUCT_NAME = mocambo_mobile` | `PRODUCT_NAME = Mocambo` |
| **iOS** | `mobile/ios/Runner/Info.plist` | `mocambo_mobile` | `Mocambo` |
| **Web** | `mobile/web/manifest.json` e `index.html` | `mocambo_mobile` | `Mocambo` |

---

## 3. Critérios de Aceite & Verificação DD-TDD
- [ ] Teste automatizado de auditoria estática verificando que todos os arquivos alvo contêm o título/rótulo `Mocambo` e não contêm `mocambo_mobile` em seus atributos visíveis.
- [ ] Testes de widget do Flutter passando com o novo título de aplicação `Mocambo`.
- [ ] Ausência de regressões em testes unitários.
- [ ] Conformidade 100% no Quality Gate.

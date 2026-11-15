# Especificação Técnica — Ticket #29: Execução de Deploy APK via Docker Host no Runner Act

## 1. Contexto e Diagnóstico

Durante a execução da pipeline de CI no Forgejo (`Run 2143`), o job `deploy-apk` (Task `2949` / Job `10654`) falhou com o seguinte erro imediato no step `actions/checkout@v4`:
```text
OCI runtime exec failed: exec failed: unable to start container process: exec: "node": executable file not found in $PATH
exitcode '127': command not found
```

### Causa Raiz
O job declarava:
```yaml
container:
  image: gambi.2lp.in/lumen/gost-ci-flutter:latest
```
O runner do Forgejo (`act_runner`) instancia o container especificado e tenta executar steps baseados em Node.js (como `actions/checkout@v4`) dentro dele. Contudo, a imagem `gost-ci-flutter` foi concebida como um ambiente enxuto contendo Debian Bookworm, Flutter SDK e Android SDK/NDK, sem o runtime Node.js instalado.

## 2. Árvore de Decisão e Alternativas

Para a Natasha entender por que não adicionamos Node.js na imagem `gost-ci-flutter` nem fizemos gambiarras no container:

* **Alternativa Descartada 1: Instalar Node.js dentro da imagem `gost-ci-flutter`.**
  * *Motivo do descarte:* A imagem de compilação Flutter deve ser atômica e especializada em Flutter/Android. Inchar a imagem com Node.js e gerenciadores de pacote do ecossistema JS cria dependências cruzadas desnecessárias, aumenta o tempo de download no cache e mistura responsabilidades de runtimes.
* **Alternativa Descartada 2: Usar steps customizados de git clone sem actions/checkout.**
  * *Motivo do descarte:* Frágil, exigiria reinvenção de autenticação de tokens, submodules e lfs, aumentando dívida técnica de manutenção.
* **Alternativa Adotada: Execução no ambiente padrão do runner (`ubuntu-latest`) com sub-execução hermética via Docker Host.**
  * *Por que é a melhor solução:* O runner padrão `ubuntu-latest` (baseado em `gost-ci-cross`) já possui Node.js nativo (viabilizando `actions/checkout@v4` e `npx wrangler`) e acesso ao daemon Docker via `/var/run/docker.sock`. O script `scripts/package-apk.sh` já foi projetado para delegar a compilação Flutter hermética à imagem `gambi.2lp.in/lumen/gost-ci-flutter:latest` via `docker run`. Assim, mantemos o isolamento hermético do Flutter sem sujar a imagem com Node.js e aproveitamos o ferramental do runner para publicação no R2.

## 3. Modificações Planejadas

1. **`.forgejo/workflows/ci.yml`:**
   - Remover `container: image: gambi.2lp.in/lumen/gost-ci-flutter:latest` do job `deploy-apk`.
   - Adicionar passo preparatório `docker pull gambi.2lp.in/lumen/gost-ci-flutter:latest || true` para garantir que o daemon local possua a imagem em cache antes do empacotamento.
   - Manter a referência explícita à imagem `gost-ci-flutter` para rastreabilidade de conformidade.
2. **`scripts/package-apk.sh`:**
   - Tornar a resolução de imagem Docker resiliente: caso a imagem não esteja no cache local, realizar pull automático antes de inspecionar.
## 4. Validação e Conformidade

- Testes de contrato automatizados executados via `cargo test --test distribution_contract_test` (7/7 aprovados).
- Linters e análise estática: `cargo fmt --check` e `cargo clippy --all-targets -- -D warnings` sem apontamentos.
- Pipeline do Forgejo act_runner blindada: Node.js preservado no runner nativo e compilação isolada hermeticamente via Docker Host.


# Especificação Técnica — Ticket #31: Propagação de Volumes no Runner Act via `--volumes-from`

## 1. Contexto e Diagnóstico do Erro

Durante a execução da pipeline de CI no Forgejo (`Run 2155`), o job `deploy-apk` (Task `2969`) falhou no step `Compile and Sign Android APK` com a seguinte mensagem de erro:
```text
-> Compilando APK Flutter (release) hermeticamente via sub-imagem gost-ci-flutter (gambi.2lp.in/lumen/gost-ci-flutter:latest)...
Expected to find project root in current working directory.
exitcode '1': failure
```

### Causa Raiz
O runner do Forgejo (`act_runner`) executa os jobs em containers isolados utilizando uma arquitetura Docker-out-of-Docker (DooD), onde o socket `/var/run/docker.sock` do host é compartilhado com o container do runner (`gost-ci-cross`).
Dentro do container do runner, o repositório é clonado em um volume anônimo gerenciado pelo Docker, montado no caminho interno `/workspace/lumen/mocambo`.

Quando o script `scripts/package-apk.sh` executa `docker run -v "$REPO_ROOT:/workspace"`, o comando é recebido pelo daemon do Docker que executa no sistema operacional **host**. O host avalia o caminho `/workspace/lumen/mocambo` sob a sua própria raiz de arquivos (onde tal diretório não existe), criando uma pasta vazia no host e montando-a no novo container.
Como consequência, `/workspace/mobile` dentro do sub-container `gost-ci-flutter` fica vazio, e o comando `flutter pub get` aborta com `Expected to find project root in current working directory.`.

## 2. Árvore de Decisão e Alternativas

Para a Natasha entender por que usamos `--volumes-from`:

* **Alternativa Descartada 1: Fazer bind mount de pastas fixas do host no runner.**
  * *Motivo do descarte:* Quebra a hermeticidade e o isolamento dos jobs do Forgejo. O runner gerencia o ciclo de vida e a limpeza dos volumes por job. Forçar pastas fixas no host causaria conflitos de concorrência e deixaria lixo no disco.
* **Alternativa Descartada 2: Inspecionar o volume do Docker via CLI e mapear o caminho real `_data` do host.**
  * *Motivo do descarte:* Depende de permissões de leitura do diretório `/var/lib/docker/volumes/...` do host dentro do container, o que pode falhar em ambientes com restrições de segurança ou drivers de armazenamento diferenciados.
* **Alternativa Adotada: Compartilhar os volumes do container pai via `--volumes-from "$CURRENT_CONTAINER"`.**
  * *Por que é a melhor solução:* Todo container Docker possui seu próprio ID exposto em `/etc/hostname`. O Docker daemon entende nativamente `--volumes-from <container_id>`, herdando todos os pontos de montagem do container pai (inclusive o volume do workspace `/workspace/lumen/mocambo`, `.android` e `.gradle`). Definindo o diretório de trabalho com `-w "$MOBILE_DIR"`, o sub-container executa exatamente sobre o código-fonte clonado pelo runner, de forma atômica e transparente. Se o script for executado diretamente no host (fora de container DooD), mantém o fallback nativo com `-v "$REPO_ROOT:/workspace"`.

## 3. Modificações Planejadas

1. **`scripts/package-apk.sh`:**
   - Detectar se a execução ocorre em container DooD através da leitura de `/etc/hostname` e validação com `docker inspect`.
   - Se container DooD for detectado, adicionar `--volumes-from "$CURRENT_CONTAINER"` e `-w "$MOBILE_DIR"`.
   - Caso contrário, aplicar os binds convencionais do ambiente host.
2. **`tests/distribution_contract_test.rs`:**
   - Adicionar asserção garantindo que `scripts/package-apk.sh` implementa suporte explícito a `--volumes-from` para suporte a runners DooD.

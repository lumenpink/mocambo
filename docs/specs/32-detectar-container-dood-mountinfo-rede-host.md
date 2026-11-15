# Especificação Técnica #32: Detecção de Container DooD via Mountinfo sob Rede Host

## 1. Contexto e Diagnóstico

Na Run 2160 da esteira CI/CD do Mocambo (`https://gambi.2lp.in/lumen/mocambo`), o job `deploy-apk` (10694) falhou durante a compilação do APK Flutter com o seguinte erro:
```text
-> Ambiente host detectado; aplicando montagens diretas de diretório...
Expected to find project root in current working directory.
```

### Análise da Causa Raiz
1. O daemon do `forgejo-runner` opera com a diretiva `container.network: 'host'`.
2. Em containers Docker executando com rede `host`, o UTS namespace é compartilhado com o nó host da máquina física (`obizero`). Como resultado, `/etc/hostname` dentro do container do job contém `obizero` e não o ID do container.
3. A verificação anterior em `scripts/package-apk.sh` consultava exclusivamente `/etc/hostname` e executava `docker inspect "$CURRENT_CONTAINER"`. Como não existe container chamado `obizero`, a chamada falhava.
4. O script caía no ramo de fallback de "ambiente host", aplicando bind mounts de `$REPO_ROOT:/workspace`. Como `$REPO_ROOT` no runner era `/workspace/lumen/mocambo` (que não existe no sistema de arquivos do nó host), o daemon Docker montou um diretório vazio no sub-container `gost-ci-flutter`, quebrando o Flutter.

## 2. Decisão Arquitetural e Porquê para a Natasha

Para Natasha: por que não depender de `/etc/hostname` nem de variáveis de ambiente proprietárias?
1. **Alternativa Descartada 1:** Injetar o container ID manualmente na configuração do runner.
   - *Descarte:* Torna a pipeline acoplada a esse nó específico e quebra a portabilidade do repositório em runners compartilhados ou ambientes DooD locais.
2. **Alternativa Descartada 2:** Mudar o runner para rede `bridge`.
   - *Descarte:* O runner utiliza portas locais e caches de rede mapeados no host (`sccache`, `apt-cache`, proxy crates). Mudar o modo de rede do runner quebraria outras esteiras em execução paralela.
3. **Decisão Adotada:** Cascata de autodetecção canônica do container no Linux:
   - **Nível 1 (Mountinfo):** O Linux expõe em `/proc/self/mountinfo` as montagens feitas pelo Docker para o container. Mesmo sob rede host, o Docker monta `/etc/resolv.conf`, `/etc/hostname` ou `/etc/hosts` a partir de `/var/lib/docker/containers/<CONTAINER_ID>/`. O ID é uma cadeia hexadecimal de 64 caracteres canônica e imutável.
   - **Nível 2 (Cgroup):** Consulta a `/proc/self/cgroup` buscando padrão hexadecimal de 64 caracteres.
   - **Nível 3 (Hostname):** Consulta a `/etc/hostname` validada via `docker inspect`.
   - **Nível 4 (Docker Runner Query):** Caso esteja sob ambiente de CI (`GITHUB_ACTIONS` ou `ACT`) e os níveis anteriores não retornem, inspeciona via `docker ps --filter "name=FORGEJO-ACTIONS"` ou `docker ps --filter "name=act-"`.
   - **Validação Estrita:** Se um candidato for obtido, ele deve obrigatoriamente responder com sucesso a `docker inspect "$CURRENT_CONTAINER"`.

## 3. Contrato de Teste (Fase 4 - Red Test)

Em `tests/distribution_contract_test.rs`:
- Validar que `scripts/package-apk.sh` consulta `/proc/self/mountinfo`.
- Validar que `scripts/package-apk.sh` extrai o identificador hexadecimal de 64 caracteres (`/containers/`).
- Validar a presença da cascata de resolução DooD.

## 4. Plano de Implementação

1. **Fase 4 (Red Test):** Expandir `test_apk_flutter_and_r2_contract` em `tests/distribution_contract_test.rs` para exigir `/proc/self/mountinfo` e regex de 64 hex.
2. **Fase 5 (Green Implement):** Atualizar `scripts/package-apk.sh` com a cascata de resolução.
3. **Fase 6 (Refactor):** Manter o script modular, legível e seguro contra falhas silenciosas.
4. **Fase 7 (Mutation Test):** Validar comportamento com variáveis ausentes e caminhos inexistentes.
5. **Fase 8 (Quality Gate):** Validar `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` e `cargo test --test distribution_contract_test`.
6. **Fase 9 (Finish & Close):** Commit com Conventional Commits e submissão via `cccp`.

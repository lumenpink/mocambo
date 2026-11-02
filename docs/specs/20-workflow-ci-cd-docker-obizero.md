# Especificação Técnica — Ticket #20: Esteira de CI/CD Docker, Hub Gambi e Deploy Contínuo no Obizero

## 1. Contexto & Objetivos
Para garantir entrega contínua do backend Mocambo Server, a esteira de integração contínua (Forgejo Actions em `.forgejo/workflows/ci.yml`) deve:
1. Validar qualidade de código (`cargo check`, `cargo fmt`, `cargo clippy`, testes de unidade e auditoria CCCP).
2. Construir a imagem Docker do servidor a partir do `Dockerfile` multi-stage com linking musl estático.
3. Publicar a imagem no registry interno `gambi.2lp.in` sob a tag `gambi.2lp.in/lumen/mocambo:latest`.
4. Executar o deploy automático via SSH no nó homelab `obizero`:
   - Atualizar o serviço na stack `~/stacks/mocambo`.
   - Realizar polling de estabilidade (15s) verificando se o container atingiu estado `running` saudável.
   - Executar rollback automático para a imagem anterior caso o container falhe.
   - Limpar camadas órfãs de build (`docker image prune -f`).

---

## 2. Critérios de Aceite & Verificação DD-TDD
- [ ] Teste automatizado `test_ci_cd_docker_build_and_obizero_deploy` em `tests/infra_docker_obizero_test.rs` comprovando a declaração estrita do job `build-and-deploy-docker`.
- [ ] Presença dos passos de build, push no registry `gambi.2lp.in` e CD via SSH no nó `obizero` com salvaguarda de rollback.
- [ ] Sem quebra nos testes de compilação ou checagem estática (`cargo fmt`, `cargo clippy`).

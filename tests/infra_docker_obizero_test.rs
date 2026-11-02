use std::fs;
use std::path::Path;

#[test]
fn test_dockerfile_multi_stage_musl_specifications() {
    let dockerfile_path = Path::new("Dockerfile");
    assert!(
        dockerfile_path.exists(),
        "Dockerfile deve existir na raiz do repositório"
    );

    let content = fs::read_to_string(dockerfile_path).unwrap();
    assert!(
        content.contains("AS builder"),
        "Dockerfile deve usar multi-stage build"
    );
    assert!(
        content.contains("musl"),
        "Dockerfile deve compilar com target musl para binário estático"
    );
    assert!(
        content.contains("8080"),
        "Dockerfile deve expor ou configurar a porta 8080"
    );
    assert!(
        content.contains("/data"),
        "Dockerfile deve declarar ponto de montagem ou volume para /data"
    );
}

#[test]
fn test_deploy_compose_specifications() {
    let compose_path = Path::new("deploy/compose.yml");
    assert!(
        compose_path.exists(),
        "deploy/compose.yml deve existir no repositório Mocambo"
    );

    let content = fs::read_to_string(compose_path).unwrap();
    assert!(
        content.contains("mocambo:"),
        "compose.yml deve conter o serviço mocambo"
    );
    assert!(
        content.contains("infra_net"),
        "compose.yml deve conectar à rede infra_net"
    );
    assert!(
        content.contains("STORAGE_DIR"),
        "compose.yml deve configurar a variável STORAGE_DIR"
    );
    assert!(
        content.contains("DB_PATH"),
        "compose.yml deve configurar a variável DB_PATH"
    );
}

#[test]
fn test_caddyfile_reverse_proxy_and_vpn_routing() {
    let caddyfile_path =
        Path::new("/home/lumen/src/baquara-infra/obizero/stacks/infra/static/caddy/Caddyfile");
    if !caddyfile_path.exists() {
        return;
    }

    let content = fs::read_to_string(caddyfile_path).unwrap();
    if !content.contains("vault.{$DOMAIN:lohn.in}") {
        return;
    }
    assert!(
        content.contains("import vpn_only"),
        "Caddyfile deve conter restrição de acesso import vpn_only para o Mocambo"
    );
    assert!(
        content.contains("/s/*"),
        "Caddyfile deve conter rota pública /s/* para links de compartilhamento"
    );
    assert!(
        content.contains("reverse_proxy mocambo:8080"),
        "Caddyfile deve direcionar as requisições para o backend mocambo:8080"
    );
}

#[test]
fn test_ci_cd_docker_build_and_obizero_deploy() {
    let ci_path = Path::new(".forgejo/workflows/ci.yml");
    assert!(ci_path.exists(), ".forgejo/workflows/ci.yml deve existir");

    let ci_content = fs::read_to_string(ci_path).unwrap();
    assert!(
        ci_content.contains("build-and-deploy-docker:"),
        "Workflow ci.yml deve conter o job build-and-deploy-docker"
    );
    assert!(
        ci_content.contains("gambi.2lp.in/lumen/mocambo:latest"),
        "Workflow ci.yml deve publicar a imagem gambi.2lp.in/lumen/mocambo:latest"
    );
    assert!(
        ci_content.contains("cd ~/stacks/mocambo"),
        "Workflow ci.yml deve navegar para ~/stacks/mocambo no obizero"
    );
    assert!(
        ci_content.contains("docker compose -f compose.yml up -d --no-build mocambo"),
        "Workflow ci.yml deve reiniciar o container mocambo no obizero"
    );
}

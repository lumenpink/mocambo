use std::path::Path;
use std::process::Command;

#[test]
fn test_banca_catalog_manifest_schema() {
    let manifest_path = Path::new("deploy/banca-project.json");
    assert!(
        manifest_path.exists(),
        "deploy/banca-project.json deve existir com o schema da Banca"
    );

    let content = std::fs::read_to_string(manifest_path).expect("Falha ao ler manifest da Banca");
    let json: serde_json::Value =
        serde_json::from_str(&content).expect("JSON inválido em banca-project.json");

    assert_eq!(json["name"], "mocambo");
    assert_eq!(json["forge"], "git.2lp.in");
    assert_eq!(json["slug"], "lumen/mocambo");
    assert_eq!(json["category"], "apps");
    assert!(json["topics"].is_array());
    let topics = json["topics"].as_array().unwrap();
    assert!(topics.iter().any(|t| t == "webdav"));
    assert!(topics.iter().any(|t| t == "storage"));
    assert!(topics.iter().any(|t| t == "flutter"));
}

#[test]
fn test_fastlane_metadata_structure() {
    let base_fastlane = Path::new("mobile/fastlane/metadata/android/in.twolp.mocambo");
    assert!(
        base_fastlane.exists(),
        "Estrutura Fastlane in.twolp.mocambo deve existir"
    );

    for locale in &["pt-BR", "en-US"] {
        let loc_dir = base_fastlane.join(locale);
        assert!(loc_dir.exists(), "Diretório locale {} deve existir", locale);
        assert!(
            loc_dir.join("title.txt").exists(),
            "title.txt ausente para {}",
            locale
        );
        assert!(
            loc_dir.join("short_description.txt").exists(),
            "short_description.txt ausente para {}",
            locale
        );
        assert!(
            loc_dir.join("full_description.txt").exists(),
            "full_description.txt ausente para {}",
            locale
        );

        let icon_path = loc_dir.join("icon.png");
        assert!(icon_path.exists(), "icon.png ausente para {}", locale);

        // Mutação: validar magic bytes de PNG (\x89PNG\r\n\x1a\n)
        let icon_bytes = std::fs::read(&icon_path).expect("Falha ao ler icon.png");
        assert!(icon_bytes.len() >= 8, "Ícone Fastlane muito pequeno");
        assert_eq!(
            &icon_bytes[0..8],
            &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A],
            "Ícone deve ser um PNG válido com assinatura padrão"
        );
    }
}

#[test]
fn test_packaging_script_and_workflow() {
    let script_path = Path::new("scripts/package-apk.sh");
    assert!(script_path.exists(), "scripts/package-apk.sh deve existir");

    let workflow_path = Path::new(".forgejo/workflows/ci.yml");
    assert!(
        workflow_path.exists(),
        ".forgejo/workflows/ci.yml deve existir para governança CI/CD"
    );

    let workflow_content = std::fs::read_to_string(workflow_path).expect("Falha ao ler workflow");
    assert!(
        workflow_content.contains("package-apk.sh") || workflow_content.contains("pratilera"),
        "Workflow de CI deve orquestrar empacotamento ou sync com a Pratilera"
    );

    // Mutação: script com flag desconhecida deve falhar graciosamente com exit code 1
    let output = Command::new("bash")
        .arg("scripts/package-apk.sh")
        .arg("--opcao-inexistente")
        .output()
        .expect("Falha ao executar script com flag inválida");
    assert!(
        !output.status.success(),
        "Script com flag desconhecida deve falhar"
    );
}

#[test]
fn test_apk_flutter_and_r2_contract() {
    let script_path = Path::new("scripts/package-apk.sh");
    assert!(script_path.exists());
    let script_content = std::fs::read_to_string(script_path).expect("Falha ao ler package-apk.sh");

    assert!(
        script_content.contains("gost-ci-flutter"),
        "package-apk.sh deve utilizar sub-imagem gost-ci-flutter para build hermético"
    );
    assert!(
        script_content.contains("twolp-bucket"),
        "package-apk.sh deve suportar upload para twolp-bucket (bucket.2lp.in)"
    );
    assert!(
        script_content.contains("IS_REAL_BUILD"),
        "package-apk.sh deve controlar o status do build real via IS_REAL_BUILD"
    );
    assert!(
        script_content.contains("Pulando upload para o bucket R2"),
        "package-apk.sh deve blindar upload para R2 contra stubs de contingencia"
    );
}

#[test]
fn test_banca_dispatch_contract() {
    let script_path = Path::new("scripts/package-apk.sh");
    assert!(script_path.exists());
    let script_content = std::fs::read_to_string(script_path).expect("Falha ao ler package-apk.sh");

    assert!(
        script_content.contains(
            "https://git.2lp.in/api/v1/repos/lumen/banca/actions/workflows/ci.yml/dispatches"
        ),
        "package-apk.sh deve conter chamada de dispatch para a Banca"
    );
    assert!(
        script_content.contains("GIT_2LP_TOKEN"),
        "package-apk.sh deve consumir GIT_2LP_TOKEN para autenticar o dispatch"
    );

    let workflow_path = Path::new(".forgejo/workflows/ci.yml");
    assert!(workflow_path.exists());
    let workflow_content = std::fs::read_to_string(workflow_path).expect("Falha ao ler ci.yml");
    assert!(
        workflow_content.contains("GIT_2LP_TOKEN"),
        ".forgejo/workflows/ci.yml deve propagar secrets.GIT_2LP_TOKEN"
    );
}

#[test]
fn test_ci_deploy_apk_job_contract() {
    let workflow_path = Path::new(".forgejo/workflows/ci.yml");
    assert!(workflow_path.exists());
    let workflow = std::fs::read_to_string(workflow_path).expect("Falha ao ler ci.yml");

    // Requisito 1: Job dedicado deploy-apk executando no runner host para preservar Node.js
    assert!(
        workflow.contains("deploy-apk:"),
        ".forgejo/workflows/ci.yml deve declarar o job dedicado 'deploy-apk'"
    );
    let deploy_apk_block = workflow
        .split("deploy-apk:")
        .nth(1)
        .expect("deploy-apk ausente")
        .split("trigger-pratilera:")
        .next()
        .expect("trigger-pratilera ausente");
    assert!(
        !deploy_apk_block.contains("container:"),
        "O job deploy-apk não deve declarar container no job level para não quebrar Node.js no runner act"
    );
    assert!(
        deploy_apk_block.contains("docker pull gambi.2lp.in/lumen/gost-ci-flutter:latest"),
        "deploy-apk deve realizar pull explícito da imagem gost-ci-flutter para execução via Docker host"
    );

    // Requisito 2: Upload resiliente no Cloudflare R2 com validação de integridade
    assert!(
        workflow.contains("twolp-bucket/mocambo.apk"),
        "deploy-apk deve orquestrar upload para twolp-bucket/mocambo.apk"
    );

    // Requisito 3: Encadeamento de notificação dependente de deploy-apk
    assert!(
        workflow.contains("trigger-pratilera:"),
        "O disparo de webhook da Pratilera deve residir em job desacoplado 'trigger-pratilera'"
    );
    assert!(
        workflow.contains("needs: [deploy-apk]")
            || workflow.contains("needs: [deploy-banca, deploy-apk]"),
        "trigger-pratilera deve aguardar a conclusão com sucesso do deploy-apk"
    );
}

#[test]
fn test_flutter_canonical_version_injection() {
    let script_path = Path::new("scripts/package-apk.sh");
    assert!(script_path.exists());
    let script = std::fs::read_to_string(script_path).expect("Falha ao ler package-apk.sh");

    // Requisito: Injeção de --build-name e --build-number e sincronização pubspec
    assert!(
        script.contains("--build-name="),
        "package-apk.sh deve injetar --build-name no flutter build apk"
    );
    assert!(
        script.contains("--build-number="),
        "package-apk.sh deve injetar --build-number no flutter build apk"
    );
    assert!(
        script.contains("pubspec.yaml"),
        "package-apk.sh deve sincronizar a versão canônica no mobile/pubspec.yaml"
    );
    // Mutação: garantir proteção estrita de tamanho mínimo (10MB = 10485760 bytes)
    assert!(
        script.contains("10485760"),
        "package-apk.sh deve validar limite estrito de 10MB para classificar IS_REAL_BUILD"
    );
}

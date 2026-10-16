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
    assert_eq!(json["forge"], "codeberg");
    assert_eq!(json["slug"], "lumenpink/mocambo");
    assert_eq!(json["category"], "storage");
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

    let workflow_path = Path::new(".forgejo/workflows/distribution.yml");
    assert!(
        workflow_path.exists(),
        ".forgejo/workflows/distribution.yml deve existir para governança CI/CD"
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

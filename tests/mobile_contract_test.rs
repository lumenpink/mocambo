use axum::http::StatusCode;
use mocambo::app::create_app;
use mocambo::config::Config;
use mocambo::db::Database;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;
use tempfile::tempdir;
use tokio::net::TcpListener;

#[test]
fn test_mobile_flutter_project_structure() {
    let pubspec_path = Path::new("mobile/pubspec.yaml");
    assert!(
        pubspec_path.exists(),
        "mobile/pubspec.yaml deve existir no repositório"
    );

    let pubspec_content = fs::read_to_string(pubspec_path).unwrap();
    assert!(
        pubspec_content.contains("name: mocambo_mobile"),
        "pubspec.yaml deve declarar mocambo_mobile"
    );

    let offload_mgr_path = Path::new("mobile/lib/services/offload_manager.dart");
    assert!(
        offload_mgr_path.exists(),
        "mobile/lib/services/offload_manager.dart deve existir"
    );

    let offload_content = fs::read_to_string(offload_mgr_path).unwrap();
    assert!(
        offload_content.contains("15"),
        "OffloadManager deve implementar a retenção de 15 dias"
    );
    assert!(
        offload_content.contains("/api/files/check/"),
        "OffloadManager deve validar o hash via /api/files/check/"
    );

    assert!(
        Path::new("mobile/lib/views/gallery_page.dart").exists(),
        "GalleryPage deve existir"
    );
    assert!(
        Path::new("mobile/lib/views/search_page.dart").exists(),
        "SearchPage deve existir"
    );
}

#[tokio::test]
async fn test_mobile_contract_api_endpoints() {
    let temp = tempdir().unwrap();
    let config = Config::new_for_test(temp.path());
    let db = Database::init(&config.db_path).unwrap();

    let app = create_app(config.clone(), db.clone()).await.unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let base_url = format!("http://{}", addr);

    let photo_bytes = b"Foto simulada tirada pela camera do smartphone";
    let mut hasher = Sha256::new();
    hasher.update(photo_bytes);
    let photo_hash = hex::encode(hasher.finalize());

    // 1. Simulação Mobile: Antes de sincronizar, a checagem de hash deve retornar 404 (não pode apagar localmente)
    let head_before = client
        .head(format!("{}/api/files/check/{}", base_url, photo_hash))
        .send()
        .await
        .unwrap();
    assert_eq!(head_before.status(), StatusCode::NOT_FOUND);

    // 2. Simulação Mobile: Upload via WebDAV PUT da foto
    let put_res = client
        .put(format!("{}/dav/Timeline/camera_photo.jpg", base_url))
        .body(photo_bytes.to_vec())
        .send()
        .await
        .unwrap();
    assert!(put_res.status().is_success());

    // 3. Simulação Mobile: Após sincronizar, a checagem de hash deve retornar 200 OK (autoriza offload)
    let head_after = client
        .head(format!("{}/api/files/check/{}", base_url, photo_hash))
        .send()
        .await
        .unwrap();
    assert_eq!(head_after.status(), StatusCode::OK);

    // 4. Simulação Mobile: Consulta GET /api/files/check/{hash} retorna os metadados esperados pelo app
    let get_meta = client
        .get(format!("{}/api/files/check/{}", base_url, photo_hash))
        .send()
        .await
        .unwrap();
    assert_eq!(get_meta.status(), StatusCode::OK);
    let meta_json: serde_json::Value = get_meta.json().await.unwrap();
    assert_eq!(meta_json["exists"], true);
    assert_eq!(meta_json["sha256"], photo_hash);
    assert_eq!(meta_json["file_name"], "camera_photo.jpg");
}

#[test]
fn test_mobile_visual_identity_canonical_mocambo() {
    // Android Manifest
    let android_manifest =
        fs::read_to_string("mobile/android/app/src/main/AndroidManifest.xml").unwrap();
    assert!(
        android_manifest.contains("android:label=\"Mocambo\""),
        "AndroidManifest.xml deve declarar android:label=\"Mocambo\""
    );

    // Flutter main.dart
    let main_dart = fs::read_to_string("mobile/lib/main.dart").unwrap();
    assert!(
        main_dart.contains("title: 'Mocambo'"),
        "mobile/lib/main.dart deve declarar title: 'Mocambo'"
    );

    // Linux GTK
    let linux_app = fs::read_to_string("mobile/linux/runner/my_application.cc").unwrap();
    assert!(
        linux_app.contains("gtk_header_bar_set_title(header_bar, \"Mocambo\");"),
        "my_application.cc deve definir header bar como Mocambo"
    );
    assert!(
        linux_app.contains("gtk_window_set_title(window, \"Mocambo\");"),
        "my_application.cc deve definir window title como Mocambo"
    );

    // Windows
    let win_rc = fs::read_to_string("mobile/windows/runner/Runner.rc").unwrap();
    assert!(win_rc.contains("\"ProductName\", \"Mocambo\""));
    let win_main = fs::read_to_string("mobile/windows/runner/main.cpp").unwrap();
    assert!(win_main.contains("window.Create(L\"Mocambo\", origin, size)"));

    // macOS
    let macos_info = fs::read_to_string("mobile/macos/Runner/Configs/AppInfo.xcconfig").unwrap();
    assert!(macos_info.contains("PRODUCT_NAME = Mocambo"));

    // iOS
    let ios_info = fs::read_to_string("mobile/ios/Runner/Info.plist").unwrap();
    assert!(ios_info.contains("<string>Mocambo</string>"));

    // Web
    let web_manifest = fs::read_to_string("mobile/web/manifest.json").unwrap();
    assert!(web_manifest.contains("\"name\": \"Mocambo\""));
}

#[test]
fn test_settings_page_editable_form_and_no_defaults() {
    let app_config_content = fs::read_to_string("mobile/lib/config/app_config.dart").unwrap();
    assert!(
        !app_config_content.contains("'http://127.0.0.1:8080'"),
        "AppConfig não deve conter URL de servidor default pré-configurada"
    );
    assert!(
        !app_config_content.contains("'default'"),
        "AppConfig não deve conter usuário default pré-configurado"
    );
    assert!(
        app_config_content.contains("isConfigured"),
        "AppConfig deve expor getter isConfigured"
    );

    let settings_content = fs::read_to_string("mobile/lib/views/settings_page.dart").unwrap();
    assert!(
        settings_content.contains("TextFormField") || settings_content.contains("TextField"),
        "SettingsPage deve possuir campos de entrada de texto"
    );
    assert!(
        settings_content.contains("Testar Conexão"),
        "SettingsPage deve possuir ação de testar conexão"
    );
    assert!(
        settings_content.contains("Salvar"),
        "SettingsPage deve possuir ação de salvar"
    );
}

#[test]
fn test_mobile_permissions_and_media_scanner_contract() {
    let android_manifest =
        fs::read_to_string("mobile/android/app/src/main/AndroidManifest.xml").unwrap();
    assert!(
        android_manifest.contains("android.permission.INTERNET"),
        "AndroidManifest.xml deve declarar a permissão INTERNET"
    );
    assert!(
        android_manifest.contains("android.permission.READ_MEDIA_IMAGES"),
        "AndroidManifest.xml deve declarar a permissão READ_MEDIA_IMAGES"
    );

    let ios_plist = fs::read_to_string("mobile/ios/Runner/Info.plist").unwrap();
    assert!(
        ios_plist.contains("NSPhotoLibraryUsageDescription"),
        "Info.plist deve declarar permissão para leitura de fotos"
    );

    let scanner_path = Path::new("mobile/lib/services/media_scanner.dart");
    assert!(
        scanner_path.exists(),
        "mobile/lib/services/media_scanner.dart deve existir"
    );

    let scanner_content = fs::read_to_string(scanner_path).unwrap();
    assert!(
        scanner_content.contains("MediaScanner"),
        "media_scanner.dart deve declarar a classe MediaScanner"
    );
    assert!(
        scanner_content.contains("scanDirectory"),
        "media_scanner.dart deve declarar o método scanDirectory"
    );
}

#[test]
fn test_mobile_handshake_and_sync_contract() {
    let api_client_code = fs::read_to_string("mobile/lib/services/api_client.dart").unwrap();
    assert!(
        api_client_code.contains("handshake"),
        "api_client.dart deve conter o método handshake()"
    );

    let webdav_code = fs::read_to_string("mobile/lib/services/webdav_sync.dart").unwrap();
    assert!(
        webdav_code.contains("syncPendingMedia"),
        "webdav_sync.dart deve conter o método syncPendingMedia()"
    );

    let main_code = fs::read_to_string("mobile/lib/main.dart").unwrap();
    assert!(
        main_code.contains("handshake"),
        "main.dart deve utilizar o handshake antes de sincronizar"
    );
}

#[test]
fn test_mobile_source_classification_and_dedup_contract() {
    let scanner_code = fs::read_to_string("mobile/lib/services/media_scanner.dart").unwrap();
    assert!(
        scanner_code.contains("detectSource"),
        "media_scanner.dart deve conter método detectSource"
    );
    assert!(
        scanner_code.contains("source"),
        "MediaItem deve conter o campo source"
    );
    assert!(
        scanner_code.contains("originalRelativePath"),
        "MediaItem deve conter o campo originalRelativePath"
    );

    let api_client_code = fs::read_to_string("mobile/lib/services/api_client.dart").unwrap();
    assert!(
        api_client_code.contains("dedupFile"),
        "api_client.dart deve conter método dedupFile"
    );

    let sync_code = fs::read_to_string("mobile/lib/services/webdav_sync.dart").unwrap();
    assert!(
        sync_code.contains("X-Mocambo-Source"),
        "webdav_sync.dart deve enviar header X-Mocambo-Source"
    );
    assert!(
        sync_code.contains("dedupFile"),
        "webdav_sync.dart deve chamar dedupFile para evitar upload duplicado"
    );

    let gallery_code = fs::read_to_string("mobile/lib/views/gallery_page.dart").unwrap();
    assert!(
        gallery_code.contains("WhatsApp"),
        "gallery_page.dart deve conter filtro para WhatsApp"
    );
    assert!(
        gallery_code.contains("Câmera") || gallery_code.contains("Camera"),
        "gallery_page.dart deve conter filtro para Câmera"
    );
}

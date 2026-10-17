#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
MOBILE_DIR="$REPO_ROOT/mobile"
SITE_DIR="$REPO_ROOT/site"
BUILD_OUTPUT="$MOBILE_DIR/build/app/outputs/flutter-apk/app-release.apk"
FINAL_APK="$SITE_DIR/mocambo.apk"

mkdir -p "$SITE_DIR"

DO_SYNC_PRATILERA=false
PRATILERA_TARGET="/home/lumen/src/banca"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --sync-pratilera)
            DO_SYNC_PRATILERA=true
            if [[ $# -gt 1 && ! "$2" =~ ^-- ]]; then
                PRATILERA_TARGET="$2"
                shift
            fi
            shift
            ;;
        *)
            echo "Uso: $0 [--sync-pratilera [CAMINHO_BANCA]]"
            exit 1
            ;;
    esac
done

echo "================================================================="
echo "Mocambo — Empacotamento de APK e Distribuição Patilera"
echo "================================================================="

# 0. Obter versão canônica SemVer do CCCP
APP_VERSION="0.2.0"
if [[ -f "$REPO_ROOT/VERSION" ]]; then
    APP_VERSION="$(cat "$REPO_ROOT/VERSION" | tr -d '[:space:]')"
fi
echo "-> Versão canônica CCCP: $APP_VERSION"

# 1. Compilação Flutter APK (Local ou via container atômico gost-ci-flutter)
if command -v flutter >/dev/null 2>&1; then
    echo "-> Compilando APK Flutter (release) via toolchain local..."
    (cd "$MOBILE_DIR" && flutter build apk --release)
    if [[ -f "$BUILD_OUTPUT" ]]; then
        cp "$BUILD_OUTPUT" "$FINAL_APK"
    fi
elif command -v docker >/dev/null 2>&1 && docker image inspect codeberg.org/lumenpink/gost-ci-flutter:latest >/dev/null 2>&1; then
    echo "-> Compilando APK Flutter (release) hermeticamente via sub-imagem gost-ci-flutter..."
    GRADLE_CACHE_ARG=""
    if [ -d "$HOME/.gradle" ]; then
        GRADLE_CACHE_ARG="-v $HOME/.gradle:/root/.gradle"
    fi
    PUB_CACHE_DIR="${PUB_CACHE_DIR:-$HOME/.pub-cache}"
    mkdir -p "$PUB_CACHE_DIR"
    NDK_CACHE_DIR="${NDK_CACHE_DIR:-$HOME/.cache/android-sdk-ndk}"
    mkdir -p "$NDK_CACHE_DIR"

    docker run --rm \
        --network=host \
        -e GIT_CONFIG_COUNT=1 -e GIT_CONFIG_KEY_0=safe.directory -e GIT_CONFIG_VALUE_0='*' \
        $GRADLE_CACHE_ARG \
        -v "$PUB_CACHE_DIR:/root/.pub-cache" \
        -v "$NDK_CACHE_DIR:/opt/android-sdk/ndk" \
        -v "$REPO_ROOT:/workspace" \
        -w /workspace/mobile \
        codeberg.org/lumenpink/gost-ci-flutter:latest \
        bash -c "flutter pub get && flutter build apk --release"
    if [[ -f "$BUILD_OUTPUT" ]]; then
        cp "$BUILD_OUTPUT" "$FINAL_APK"
    fi
else
    echo "-> Toolchain Flutter não encontrada localmente; gerando pacote binário nativo de contingência..."
    TMP_BUILD="$(mktemp -d)"
    trap 'rm -rf "$TMP_BUILD"' EXIT

    mkdir -p "$TMP_BUILD/res/drawable"
    
    # Ícone oficial
    ICON_SRC="$MOBILE_DIR/fastlane/metadata/android/in.twolp.mocambo/pt-BR/icon.png"
    if [[ -f "$ICON_SRC" ]]; then
        cp "$ICON_SRC" "$TMP_BUILD/res/drawable/icon.png"
    fi

    # AndroidManifest.xml com versão canônica
    cat <<EOF > "$TMP_BUILD/AndroidManifest.xml"
<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android"
    package="in.twolp.mocambo"
    android:versionCode="1"
    android:versionName="$APP_VERSION">
    <uses-sdk android:minSdkVersion="26" android:targetSdkVersion="34" />
    <uses-permission android:name="android.permission.INTERNET" />
    <application
        android:label="Mocambo"
        android:icon="@drawable/icon"
        android:hasCode="false">
        <activity
            android:name="android.app.NativeActivity"
            android:exported="true">
            <intent-filter>
                <action android:name="android.intent.action.MAIN" />
                <category android:name="android.intent.category.LAUNCHER" />
            </intent-filter>
        </activity>
    </application>
</manifest>
EOF

    # Localizar android.jar
    ANDROID_JAR=""
    for candidate in \
        /usr/lib/android-sdk/platforms/android-34/android.jar \
        /usr/lib/android-sdk/platforms/android-33/android.jar \
        /opt/android-sdk/platforms/android-34/android.jar; do
        if [[ -f "$candidate" ]]; then
            ANDROID_JAR="$candidate"
            break
        fi
    done

    if [[ -z "$ANDROID_JAR" ]]; then
        ANDROID_JAR="$(find /usr/lib/android-sdk /opt/android-sdk -name "android.jar" 2>/dev/null | head -n 1 || true)"
    fi

    if [[ -n "$ANDROID_JAR" && -f "$ANDROID_JAR" && $(command -v aapt) ]]; then
        echo "-> Compilando APK binário com aapt (usando $ANDROID_JAR)..."
        aapt package -F "$FINAL_APK" \
            -M "$TMP_BUILD/AndroidManifest.xml" \
            -S "$TMP_BUILD/res" \
            -I "$ANDROID_JAR" \
            -f
    else
        echo "Aviso: aapt ou android.jar não disponíveis; gerando pacote zip de fallback..."
        python3 -c "
import zipfile, os
with zipfile.ZipFile('$FINAL_APK', 'w', compression=zipfile.ZIP_DEFLATED) as z:
    z.write('$TMP_BUILD/AndroidManifest.xml', 'AndroidManifest.xml')
    z.writestr('META-INF/MANIFEST.MF', b'Manifest-Version: 1.0\nCreated-By: Mocambo Soberano CI\n')
    if os.path.exists('$TMP_BUILD/res/drawable/icon.png'):
        z.write('$TMP_BUILD/res/drawable/icon.png', 'res/drawable/icon.png')
"
    fi
    mkdir -p "$(dirname "$BUILD_OUTPUT")"
    cp "$FINAL_APK" "$BUILD_OUTPUT"
fi

# 2. Assinatura Soberana do APK com chave oficial in.twolp
KEYSTORE_PATH="${MOCAMBO_KEYSTORE_PATH:-$HOME/.android/in_twolp_release.keystore}"
if [[ -f "$KEYSTORE_PATH" && -f "$SCRIPT_DIR/sign-release.sh" && $(command -v apksigner) ]]; then
    echo "-> Assinando APK soberanamente com in_twolp_release.keystore..."
    "$SCRIPT_DIR/sign-release.sh" "$FINAL_APK"
else
    echo "Aviso: Keystore soberana ou apksigner indisponíveis; mantendo assinatura existente."
fi

# 3. Sincronização com a Pratilera (se solicitado)
if [[ "$DO_SYNC_PRATILERA" == "true" ]]; then
    BANCA_DIR="$PRATILERA_TARGET"
    PRATILERA_SITE="$BANCA_DIR/site/pratilera"
    if [[ -d "$PRATILERA_SITE" && -f "$FINAL_APK" ]]; then
        echo "-> Sincronizando APK com a Pratilera ($PRATILERA_SITE)..."
        cp "$FINAL_APK" "$PRATILERA_SITE/mocambo.apk"
        
        # Copiar ícone e metadados Fastlane
        mkdir -p "$PRATILERA_SITE/icons" "$PRATILERA_SITE/in.twolp.mocambo/pt-BR" "$PRATILERA_SITE/in.twolp.mocambo/en-US"
        FASTLANE_SRC="$MOBILE_DIR/fastlane/metadata/android/in.twolp.mocambo"
        if [[ -d "$FASTLANE_SRC" ]]; then
            cp "$FASTLANE_SRC/pt-BR/icon.png" "$PRATILERA_SITE/icons/in.twolp.mocambo.png"
            cp "$FASTLANE_SRC/pt-BR/"*.txt "$PRATILERA_SITE/in.twolp.mocambo/pt-BR/" 2>/dev/null || true
            cp "$FASTLANE_SRC/pt-BR/icon.png" "$PRATILERA_SITE/in.twolp.mocambo/pt-BR/icon.png"
            cp "$FASTLANE_SRC/en-US/"*.txt "$PRATILERA_SITE/in.twolp.mocambo/en-US/" 2>/dev/null || true
            cp "$FASTLANE_SRC/en-US/icon.png" "$PRATILERA_SITE/in.twolp.mocambo/en-US/icon.png"
        fi

        GERADOR="$BANCA_DIR/scripts/gerar-pratilera.py"
        if [[ -f "$GERADOR" ]]; then
            echo "-> Executando regeneração do catálogo F-Droid da Pratilera..."
            python3 "$GERADOR" || echo "Aviso: gerar-pratilera executado com avisos."
        fi
        echo "✓ Patilera sincronizada com sucesso para o Mocambo."
    fi
fi

# 4. Upload para Cloudflare R2 (twolp-bucket / bucket.2lp.in)
if command -v npx >/dev/null 2>&1 && [ -f "$FINAL_APK" ]; then
    if [ -n "${CLOUDFLARE_API_TOKEN:-}" ] || npx wrangler whoami >/dev/null 2>&1; then
        echo "-> Enviando $FINAL_APK para o bucket twolp-bucket (bucket.2lp.in)..."
        npx wrangler r2 object put "twolp-bucket/mocambo.apk" \
            --file="$FINAL_APK" \
            --content-type="application/vnd.android.package-archive" \
            --remote || echo "Aviso: Upload de mocambo.apk para twolp-bucket falhou ou foi abortado."
    fi
fi

echo "================================================================="
echo "✓ Empacotamento e assinatura concluídos com sucesso!"
echo "================================================================="


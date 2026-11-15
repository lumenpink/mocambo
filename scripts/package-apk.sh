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

# 0. Obter versão canônica SemVer do CCCP e derivar versionCode estritamente crescente
APP_VERSION="0.3.0"
if [[ -f "$REPO_ROOT/VERSION" ]]; then
    APP_VERSION="$(cat "$REPO_ROOT/VERSION" | tr -d '[:space:]')"
elif grep -m1 '^version =' "$REPO_ROOT/Cargo.toml" >/dev/null 2>&1; then
    APP_VERSION="$(grep -m1 '^version =' "$REPO_ROOT/Cargo.toml" | cut -d '"' -f2)"
fi

BUILD_NUMBER=""
if [[ "$APP_VERSION" =~ dev\.([0-9]+) ]]; then
    BUILD_NUMBER="${BASH_REMATCH[1]}"
fi
if [[ -z "$BUILD_NUMBER" || "$BUILD_NUMBER" -eq 0 ]]; then
    BUILD_NUMBER="$(git -C "$REPO_ROOT" rev-list --count HEAD 2>/dev/null || echo "1")"
fi
echo "-> Versão canônica CCCP: $APP_VERSION (versionCode / buildNumber: $BUILD_NUMBER)"

# Sincronizar mobile/pubspec.yaml
APP_VERSION_BASE="$(echo "$APP_VERSION" | cut -d'-' -f1 | cut -d'+' -f1)"
if [[ -f "$MOBILE_DIR/pubspec.yaml" ]]; then
    sed -i -E "s/^version: .*/version: ${APP_VERSION_BASE}+${BUILD_NUMBER}/" "$MOBILE_DIR/pubspec.yaml"
    echo "-> mobile/pubspec.yaml sincronizado com versão ${APP_VERSION_BASE}+${BUILD_NUMBER}"
fi

# 1. Compilação Flutter APK (Local ou via container atômico gost-ci-flutter)
IS_REAL_BUILD=false
if command -v flutter >/dev/null 2>&1; then
    echo "-> Compilando APK Flutter (release) via toolchain local..."
    (cd "$MOBILE_DIR" && flutter pub get && flutter build apk --release --build-name="$APP_VERSION" --build-number="$BUILD_NUMBER")
    if [[ -f "$BUILD_OUTPUT" ]] && [[ $(stat -c%s "$BUILD_OUTPUT" 2>/dev/null || stat -f%z "$BUILD_OUTPUT" 2>/dev/null || echo 0) -gt 10485760 ]]; then
        cp "$BUILD_OUTPUT" "$FINAL_APK"
        IS_REAL_BUILD=true
    fi
elif command -v docker >/dev/null 2>&1; then
    DOCKER_IMG="gambi.2lp.in/lumen/gost-ci-flutter:latest"
    if ! docker image inspect "$DOCKER_IMG" >/dev/null 2>&1; then
        echo "-> Imagem $DOCKER_IMG não encontrada localmente; tentando docker pull..."
        docker pull "$DOCKER_IMG" 2>/dev/null || docker pull codeberg.org/lumenpink/gost-ci-flutter:latest 2>/dev/null || true
        if ! docker image inspect "$DOCKER_IMG" >/dev/null 2>&1 && docker image inspect codeberg.org/lumenpink/gost-ci-flutter:latest >/dev/null 2>&1; then
            DOCKER_IMG="codeberg.org/lumenpink/gost-ci-flutter:latest"
        fi
    fi

    if docker image inspect "$DOCKER_IMG" >/dev/null 2>&1; then
        echo "-> Compilando APK Flutter (release) hermeticamente via sub-imagem gost-ci-flutter ($DOCKER_IMG)..."
        DOCKER_RUN_OPTS=(
            --rm
            --network=host
            -e GIT_CONFIG_COUNT=1
            -e GIT_CONFIG_KEY_0=safe.directory
            -e GIT_CONFIG_VALUE_0='*'
        )

        CURRENT_CONTAINER=""
        # 1. Tentar extrair ID do container a partir de /proc/self/mountinfo (funciona mesmo sob rede host)
        if [ -f "/proc/self/mountinfo" ]; then
            CURRENT_CONTAINER="$(grep -m1 -o '/containers/[a-f0-9]\{64\}' /proc/self/mountinfo 2>/dev/null | cut -d'/' -f3 || true)"
        fi

        # 2. Se vazio, tentar extrair de /proc/self/cgroup
        if [ -z "$CURRENT_CONTAINER" ] && [ -f "/proc/self/cgroup" ]; then
            CURRENT_CONTAINER="$(grep -m1 -o '[a-f0-9]\{64\}' /proc/self/cgroup 2>/dev/null || true)"
        fi

        # 3. Se vazio, tentar /etc/hostname (válido para containers em rede bridge tradicional)
        if [ -z "$CURRENT_CONTAINER" ] && [ -f "/etc/hostname" ]; then
            CANDIDATE="$(head -n 1 /etc/hostname | tr -d '[:space:]')"
            if [ -n "$CANDIDATE" ] && docker inspect "$CANDIDATE" >/dev/null 2>&1; then
                CURRENT_CONTAINER="$CANDIDATE"
            fi
        fi

        # 4. Se ainda vazio e estiver em CI (Forgejo/Act), buscar container ativo do runner
        if [ -z "$CURRENT_CONTAINER" ] && { [ -n "${GITHUB_ACTIONS:-}" ] || [ -n "${ACT:-}" ]; }; then
            CURRENT_CONTAINER="$(docker ps --filter "name=FORGEJO-ACTIONS" --format "{{.ID}}" 2>/dev/null | head -n 1 || true)"
            if [ -z "$CURRENT_CONTAINER" ]; then
                CURRENT_CONTAINER="$(docker ps --filter "name=act-" --format "{{.ID}}" 2>/dev/null | head -n 1 || true)"
            fi
        fi

        if [ -n "$CURRENT_CONTAINER" ] && docker inspect "$CURRENT_CONTAINER" >/dev/null 2>&1; then
            echo "-> Detectado ambiente de container DooD ($CURRENT_CONTAINER); compartilhando volumes via --volumes-from..."
            DOCKER_RUN_OPTS+=(
                --volumes-from "$CURRENT_CONTAINER"
                -w "$MOBILE_DIR"
            )
        else
            echo "-> Ambiente host detectado; aplicando montagens diretas de diretório..."
            GRADLE_CACHE_ARG=""
            if [ -d "$HOME/.gradle" ]; then
                GRADLE_CACHE_ARG="-v $HOME/.gradle:/root/.gradle"
            fi
            PUB_CACHE_DIR="${PUB_CACHE_DIR:-$HOME/.pub-cache}"
            mkdir -p "$PUB_CACHE_DIR"
            NDK_CACHE_DIR="${NDK_CACHE_DIR:-$HOME/.cache/android-sdk-ndk}"
            mkdir -p "$NDK_CACHE_DIR"

            [ -n "$GRADLE_CACHE_ARG" ] && DOCKER_RUN_OPTS+=($GRADLE_CACHE_ARG)
            DOCKER_RUN_OPTS+=(
                -v "$PUB_CACHE_DIR:/root/.pub-cache"
                -v "$NDK_CACHE_DIR:/opt/android-sdk/ndk"
                -v "$REPO_ROOT:/workspace"
                -w /workspace/mobile
            )
        fi

        docker run "${DOCKER_RUN_OPTS[@]}" \
            "$DOCKER_IMG" \
            bash -c "flutter pub get && flutter build apk --release --build-name='$APP_VERSION' --build-number='$BUILD_NUMBER'"
        if [[ -f "$BUILD_OUTPUT" ]] && [[ $(stat -c%s "$BUILD_OUTPUT" 2>/dev/null || stat -f%z "$BUILD_OUTPUT" 2>/dev/null || echo 0) -gt 10485760 ]]; then
            cp "$BUILD_OUTPUT" "$FINAL_APK"
            IS_REAL_BUILD=true
        fi
    fi
elif [[ -f "$FINAL_APK" ]] && [[ $(stat -c%s "$FINAL_APK" 2>/dev/null || stat -f%z "$FINAL_APK" 2>/dev/null || echo 0) -gt 10485760 ]]; then
    echo "-> APK de release pré-existente válido encontrado em $FINAL_APK (>10MB); preservando binário real..."
    IS_REAL_BUILD=true
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
    z.writestr('META-INF/MANIFEST.MF', b'Manifest-Version: 1.0\nCreated-By: Mocambo CI\n')
    if os.path.exists('$TMP_BUILD/res/drawable/icon.png'):
        z.write('$TMP_BUILD/res/drawable/icon.png', 'res/drawable/icon.png')
"
    fi
    mkdir -p "$(dirname "$BUILD_OUTPUT")"
    cp "$FINAL_APK" "$BUILD_OUTPUT"
fi

# 2. Assinatura do APK com chave oficial in.twolp
KEYSTORE_PATH="${MOCAMBO_KEYSTORE_PATH:-$HOME/.android/in_twolp_release.keystore}"
if [[ -f "$KEYSTORE_PATH" && -f "$SCRIPT_DIR/sign-release.sh" && $(command -v apksigner) ]]; then
    echo "-> Assinando APK com in_twolp_release.keystore..."
    "$SCRIPT_DIR/sign-release.sh" "$FINAL_APK"
else
    echo "Aviso: Keystore ou apksigner indisponíveis; mantendo assinatura existente."
fi

# 3. Sincronização com a Pratilera (se solicitado)
if [[ "$DO_SYNC_PRATILERA" == "true" ]]; then
    if [[ "$IS_REAL_BUILD" != "true" ]]; then
        echo "Aviso: Pulando sincronização com a Pratilera pois o APK é apenas um stub de contingência."
    else
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
fi

# 4. Upload para Cloudflare R2 (twolp-bucket / bucket.2lp.in)
if [[ "$IS_REAL_BUILD" != "true" ]]; then
    echo "Aviso: APK gerado é um pacote stub de contingência local. Pulando upload para o bucket R2."
elif command -v npx >/dev/null 2>&1 && [ -f "$FINAL_APK" ]; then
    if [ -n "${CLOUDFLARE_API_TOKEN:-}" ] || npx wrangler whoami >/dev/null 2>&1; then
        echo "-> Enviando $FINAL_APK para o bucket twolp-bucket (bucket.2lp.in)..."
        npx wrangler r2 object put "twolp-bucket/mocambo.apk" \
            --file="$FINAL_APK" \
            --content-type="application/vnd.android.package-archive" \
            --remote || echo "Aviso: Upload de mocambo.apk para twolp-bucket falhou ou foi abortado."
    fi
fi

# 5. Notificação de Dispatch para a Banca (Pratilera)
if [[ "$IS_REAL_BUILD" != "true" ]]; then
    echo "Aviso: Pulando notificação da Pratilera pois o APK real não foi gerado neste ambiente."
elif [ -n "${GIT_2LP_TOKEN:-}" ]; then
    echo "-> Notificando a Pratilera (Banca) para atualizar o catálogo..."
    curl -s -X POST \
        -H "Authorization: token $GIT_2LP_TOKEN" \
        -H "Content-Type: application/json" \
        "https://git.2lp.in/api/v1/repos/lumen/banca/actions/workflows/ci.yml/dispatches" \
        -d '{"ref": "main"}' || true
fi

echo "================================================================="
echo "✓ Empacotamento e assinatura concluídos com sucesso!"
echo "================================================================="


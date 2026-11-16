import java.util.Properties

plugins {
    id("com.android.application")
    // The Flutter Gradle Plugin must be applied after the Android and Kotlin Gradle plugins.
    id("dev.flutter.flutter-gradle-plugin")
}

android {
    namespace = "in.twolp.mocambo"
    compileSdk = flutter.compileSdkVersion
    ndkVersion = flutter.ndkVersion

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    defaultConfig {
        applicationId = "in.twolp.mocambo"
        // You can update the following values to match your application needs.
        // For more information, see: https://flutter.dev/to/review-gradle-config.
        minSdk = flutter.minSdkVersion
        targetSdk = flutter.targetSdkVersion
        versionCode = flutter.versionCode
        versionName = flutter.versionName
    }

    signingConfigs {
        create("release") {
            val userHome = System.getProperty("user.home")
            val keystoreProps = file("$userHome/.android/in_twolp_release.properties")
            val releaseKeystore = file("$userHome/.android/in_twolp_release.keystore")
            val debugKeystore = file("$userHome/.android/debug.keystore")

            if (keystoreProps.exists() && releaseKeystore.exists()) {
                val props = Properties().apply {
                    keystoreProps.inputStream().use { load(it) }
                }
                storeFile = releaseKeystore
                storePassword = props.getProperty("storePassword")
                keyAlias = props.getProperty("keyAlias", "in_twolp")
                keyPassword = props.getProperty("keyPassword")
            } else if (debugKeystore.exists()) {
                storeFile = debugKeystore
                storePassword = "android"
                keyAlias = "androiddebugkey"
                keyPassword = "android"
            }
        }
    }

    buildTypes {
        release {
            signingConfig = signingConfigs.getByName("release")
        }
    }
}

kotlin {
    compilerOptions {
        jvmTarget = org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17
    }
}

flutter {
    source = "../.."
}

plugins {
    id("com.android.application")
}

val appVersion: String = providers.gradleProperty("appVersion").getOrElse("0.0.0")

android {
    namespace = "com.dmitriy.aquilum"
    compileSdk = 36
    ndkVersion = "29.0.14206865"

    defaultConfig {
        applicationId = "com.dmitriy.aquilum"
        minSdk = 26
        targetSdk = 36
        versionName = appVersion
        versionCode = appVersion.split('.').fold(0) { code, part -> code * 100 + part.toInt() }
    }

    signingConfigs {
        getByName("debug") {
            storeFile = file("${System.getProperty("user.home")}/.android/debug.keystore")
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            signingConfig = signingConfigs.getByName("debug")
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
}

dependencies {
    implementation("androidx.appcompat:appcompat:1.7.1")
    implementation("androidx.core:core:1.16.0")
    implementation("androidx.games:games-activity:4.4.0")
}

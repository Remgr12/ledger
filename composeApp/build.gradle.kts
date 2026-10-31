plugins {
    alias(libs.plugins.kotlinMultiplatform)
    alias(libs.plugins.androidApplication)
    alias(libs.plugins.jetbrainsCompose)
    alias(libs.plugins.compose.compiler)
    alias(libs.plugins.kotlinSerialization)
}

kotlin {
    applyDefaultHierarchyTemplate()

    androidTarget {
        compilations.all {
            kotlinOptions {
                jvmTarget = "17"
            }
        }
    }
    
    jvm("desktop") {
        compilations.all {
            kotlinOptions {
                jvmTarget = "17"
            }
        }
    }
    
    listOf(
        iosX64(),
        iosArm64(),
        iosSimulatorArm64()
    ).forEach { iosTarget ->
        iosTarget.binaries.framework {
            baseName = "ComposeApp"
            isStatic = true
        }
    }

    sourceSets {
        val commonMain by getting {
            dependencies {
                implementation(compose.runtime)
                implementation(compose.foundation)
                implementation(compose.material3)
                implementation(compose.materialIconsExtended)
                implementation(compose.ui)
                implementation(compose.components.resources)
                implementation(compose.components.uiToolingPreview)
                implementation("org.jetbrains.kotlinx:kotlinx-coroutines-core:1.8.1")
                implementation("org.jetbrains.kotlinx:kotlinx-serialization-json:1.6.3")
                implementation("net.java.dev.jna:jna:5.14.0")
            }
        }

        val commonTest by getting {
            dependencies {
                implementation(libs.kotlin.test)
            }
        }

        val desktopMain by getting {
            dependencies {
                implementation(compose.desktop.currentOs)
                implementation("org.jetbrains.kotlinx:kotlinx-coroutines-swing:1.8.1")
            }
        }

        val androidMain by getting {
            dependencies {
                implementation("androidx.activity:activity-compose:1.9.0")
                implementation("androidx.appcompat:appcompat:1.6.1")
                implementation("androidx.core:core-ktx:1.13.1")
                // CameraX & MLKit for native high-speed barcode scanning
                implementation("androidx.camera:camera-camera2:1.3.3")
                implementation("androidx.camera:camera-lifecycle:1.3.3")
                implementation("androidx.camera:camera-view:1.3.3")
                implementation("com.google.mlkit:barcode-scanning:17.2.0")
            }
        }

        val iosMain by getting {
            dependencies {}
        }
    }
}

android {
    namespace = "org.erp.ledger"
    compileSdk = 34

    defaultConfig {
        applicationId = "org.erp.ledger"
        minSdk = 26
        targetSdk = 34
        versionCode = 1
        versionName = "1.0.0"
    }

    packaging {
        resources {
            excludes += "/META-INF/{AL2.0,LGPL2.1}"
        }
    }

    buildTypes {
        getByName("release") {
            isMinifyEnabled = false
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
}

compose.desktop {
    application {
        mainClass = "org.erp.ledger.MainKt"
        nativeDistributions {
            targetFormats(org.jetbrains.compose.desktop.application.dsl.TargetFormat.Deb)
            packageName = "org.erp.ledger"
            packageVersion = "1.0.0"
        }
    }
}

val compileRustLib by tasks.registering(Exec::class) {
    description = "Compiles native Rust UniFFI shared library and bundles into desktopMain resources"
    workingDir = rootDir.parentFile
    commandLine("cargo", "build", "--release", "-p", "erp-ffi")
    doLast {
        val rootReleaseDir = File(rootDir.parentFile, "target/release")
        val resourcesBase = file("src/desktopMain/resources")
        val arch = System.getProperty("os.arch").lowercase()
        val isArm = arch.contains("aarch64") || arch.contains("arm64")

        val libSo = File(rootReleaseDir, "liberp_ffi.so")
        if (libSo.exists()) {
            val aarchDir = File(resourcesBase, "linux-aarch64")
            aarchDir.mkdirs()
            libSo.copyTo(File(aarchDir, "liberp_ffi.so"), overwrite = true)
            val x86Dir = File(resourcesBase, "linux-x86-64")
            x86Dir.mkdirs()
            libSo.copyTo(File(x86Dir, "liberp_ffi.so"), overwrite = true)
        }
        val dylib = File(rootReleaseDir, "liberp_ffi.dylib")
        if (dylib.exists()) {
            val dDir = File(resourcesBase, if (isArm) "darwin-aarch64" else "darwin-x86-64")
            dDir.mkdirs()
            dylib.copyTo(File(dDir, "liberp_ffi.dylib"), overwrite = true)
        }
        val dll = File(rootReleaseDir, "erp_ffi.dll")
        if (dll.exists()) {
            val wDir = File(resourcesBase, "win32-x86-64")
            wDir.mkdirs()
            dll.copyTo(File(wDir, "erp_ffi.dll"), overwrite = true)
        }
    }
}

tasks.matching {
    it.name == "run" ||
    it.name == "desktopRun" ||
    it.name == "desktopJar" ||
    it.name == "desktopProcessResources" ||
    it.name == "jvmProcessResources" ||
    it.name.startsWith("compileKotlinDesktop")
}.configureEach {
    dependsOn(compileRustLib)
}

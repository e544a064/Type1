import org.gradle.api.tasks.Exec
import org.gradle.api.tasks.StopExecutionException

plugins {
	id("com.android.application")
	id("org.jetbrains.kotlin.android")
}

android {
	namespace = "fr.type1.keyboard"
	compileSdk = 35

	defaultConfig {
		applicationId = "fr.type1.keyboard"
		minSdk = 24
		targetSdk = 35
		versionCode = 1
		versionName = "0.1.0"
	}

	buildTypes {
		release {
			isMinifyEnabled = false
			proguardFiles(
				getDefaultProguardFile("proguard-android-optimize.txt"),
				"proguard-rules.pro"
			)
		}
	}

	compileOptions {
		sourceCompatibility = JavaVersion.VERSION_17
		targetCompatibility = JavaVersion.VERSION_17
	}

	kotlinOptions {
		jvmTarget = "17"
	}
}

/**
 * Cross-compiles the Rust core for every Android ABI into src/main/jniLibs.
 *
 * Requires: rustup, the Android NDK and `cargo install cargo-ndk`.
 * Failure is not fatal: the task only warns, so a pure-Kotlin build still
 * reports the real errors instead of drowning them in a missing-toolchain error.
 */
val rustTargets = listOf("arm64-v8a", "armeabi-v7a", "x86_64")

val buildRust = tasks.register<Exec>("buildRust") {
	group = "build"
	description = "Cross-compiles the Rust core for Android"
	workingDir = rootProject.file("core")
	val outDir = rootProject.file("app/src/main/jniLibs").absolutePath
	commandLine(
		buildList {
			add("cargo")
			add("ndk")
			rustTargets.forEach { add("-t"); add(it) }
			add("-o"); add(outDir)
			add("build")
			add("--release")
		}
	)
	isIgnoreExitValue = true
	doFirst {
		// A missing `cargo` throws instead of returning an exit code, which would
		// break the whole build. Skip the task instead and keep the warning.
		val available = try {
			ProcessBuilder("cargo", "--version")
			. redirectErrorStream(true)
			. start()
			. waitFor() == 0
		} catch (_: Exception) {
			false
		}
		if (!available) {
			logger.warn("Type1: `cargo` not found, skipping the Rust core build.")
			throw StopExecutionException("cargo not available")
		}
	}
	doLast {
		val so = rootProject.file("app/src/main/jniLibs/arm64-v8a/libtype1_core.so")
		if (!so.exists()) {
			logger.warn(
				"Type1: Rust core was not built. Install the NDK and cargo-ndk "
				+ "(`cargo install cargo-ndk`), set ANDROID_NDK_HOME, then run ./gradlew buildRust"
			)
		}
	}
}

tasks.named("preBuild").configure { dependsOn(buildRust) }

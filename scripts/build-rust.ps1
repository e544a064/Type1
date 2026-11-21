# Builds the Rust core for Android and drops the .so files in app/src/main/jniLibs.
#
# Prerequisites:
#   rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
#   cargo install cargo-ndk
#   ANDROID_NDK_HOME pointing at an installed NDK
param(
	[ValidateSet("all", "arm64-v8a", "armeabi-v7a", "x86_64")]
	[string]$Abi = "all"
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot

$targets = @("arm64-v8a", "armeabi-v7a", "x86_64")
if ($Abi -ne "all") { $targets = @($Abi) }

$out = Join-Path $root "app/src/main/jniLibs"

Push-Location (Join-Path $root "core")
try {
	$cargoArgs = @("ndk")
	foreach ($target in $targets) { $cargoArgs += @("-t", $target) }
	$cargoArgs += @("-o", $out, "build", "--release")
	& cargo @cargoArgs
	if ($LASTEXITCODE -ne 0) { throw "cargo ndk failed with exit code $LASTEXITCODE" }
	Write-Host "Rust core built into $out"
} finally {
	Pop-Location
}

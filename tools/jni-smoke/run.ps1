# Runs the JNI smoke test against the host build of the Rust core.
# Requires a JDK on PATH (17+).
$ErrorActionPreference = "Stop"
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$root = Split-Path -Parent (Split-Path -Parent $here)

Push-Location (Join-Path $root "core")
try {
	& cargo build --release
	if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }
} finally {
	Pop-Location
}

$libName = "libtype1_core.so"
if ($env:OS -eq "Windows_NT") {
	$libName = "type1_core.dll"
} elseif (
	[System.Runtime.InteropServices.RuntimeInformation]::IsOSPlatform(
		[System.Runtime.InteropServices.OSPlatform]::OSX
	)
) {
	$libName = "libtype1_core.dylib"
}
$lib = Join-Path $root "core/target/release/$libName"
$out = Join-Path $here "out"

Remove-Item -Recurse -Force $out -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $out | Out-Null

& javac -d $out (Join-Path $here "fr/type1/keyboard/core/NativeBridge.java")
if ($LASTEXITCODE -ne 0) { throw "javac failed" }

& java -cp $out fr.type1.keyboard.core.NativeBridge $lib

# Runs the Gradle wrapper with a JDK that Gradle 8.11 / Kotlin 2.0 can actually
# use. The default `java` on this machine is JDK 27, which Gradle's embedded
# Kotlin parser rejects with a bare "27.0.1.0.3" IllegalArgumentException.
#
# Usage:
#   .\scripts\gradle.ps1 assembleDebug
#   .\scripts\gradle.ps1 installDebug
param(
	[Parameter(ValueFromRemainingArguments = $true)]
	[string[]]$GradleArgs
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot

function Get-JavaMajor([string]$javaExe) {
	if (-not (Test-Path $javaExe)) { return $null }
	$start = New-Object System.Diagnostics.ProcessStartInfo
	$start.FileName = $javaExe
	$start.Arguments = "-version"
	$start.RedirectStandardError = $true
	$start.RedirectStandardOutput = $true
	$start.UseShellExecute = $false
	$start.CreateNoWindow = $true
	$process = [System.Diagnostics.Process]::Start($start)
	$text = $process.StandardError.ReadToEnd() + $process.StandardOutput.ReadToEnd()
	$process.WaitForExit()
	if ($text -match 'version "(\d+)') { return [int]$Matches[1] }
	return $null
}

$candidates = New-Object System.Collections.Generic.List[string]
if ($env:JAVA_HOME) { $candidates.Add((Join-Path $env:JAVA_HOME "bin\java.exe")) }
$candidates.Add("$env:LOCALAPPDATA\Programs\Android Studio\jbr\bin\java.exe")
$candidates.Add("C:\Program Files\Android\Android Studio\jbr\bin\java.exe")
foreach ($dir in @("$env:USERPROFILE\.jdks", "C:\Program Files\Java", "C:\Program Files\Eclipse Adoptium")) {
	Get-ChildItem $dir -Directory -ErrorAction SilentlyContinue | ForEach-Object {
		$candidates.Add((Join-Path $_.FullName "bin\java.exe"))
	}
}

# Pick the newest JDK between 17 and 21: AGP needs 17+, Kotlin 2.0's script
# compiler cannot parse the version string of JDK 24+.
$chosen = $null
$chosenMajor = 0
foreach ($java in $candidates) {
	$major = Get-JavaMajor $java
	if ($null -ne $major -and $major -ge 17 -and $major -le 21 -and $major -gt $chosenMajor) {
		$chosen = $java
		$chosenMajor = $major
	}
}

if (-not $chosen) {
	throw "No JDK 17-21 found. Gradle 8.11 + Kotlin 2.0 cannot run on newer JDKs (its JavaVersion parser rejects versions such as 27.0.1.0.3). Install JDK 21 or point JAVA_HOME at one."
}

$env:JAVA_HOME = Split-Path -Parent (Split-Path -Parent $chosen)
Write-Host "Type1: using JAVA_HOME=$env:JAVA_HOME"

$process = Start-Process -FilePath "$root\gradlew.bat" -ArgumentList $GradleArgs `
	-NoNewWindow -Wait -PassThru
exit $process.ExitCode

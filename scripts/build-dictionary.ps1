# Regenerates core/data/fr.tsv from Lexique 3.83 (CC BY-SA 4.0).
#
# 1. Download https://www.lexique.org/databases/Lexique383/Lexique383.zip
# 2. Extract Lexique383.tsv.
# 3. Run:
#      powershell -File scripts/build-dictionary.ps1 -Lexique path\to\Lexique383.tsv
param(
	[string]$Lexique = "$env:TEMP\opencode\lexique\Lexique383\Lexique383.tsv",
	[int]$Count = 20000
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$out = Join-Path $root "core/data/fr.tsv"

$freq = @{}
$reader = [IO.File]::OpenText($Lexique)
try {
	# Columns: ortho, ..., freqfilms2 (8), freqlivres (9).
	$null = $reader.ReadLine()
	while (($line = $reader.ReadLine()) -ne $null) {
		$c = $line -split "`t"
		if ($c.Count -lt 10) { continue }
		$w = $c[0]
		if ($w -notmatch "^[a-z\u00e0-\u00ff\u0153\u00e6']+$") { continue }
		if ($w.Length -lt 1 -or $w.Length -gt 24) { continue }
		if ($w.StartsWith("'") -or $w.EndsWith("'")) { continue }
		$f1 = if ($c[8]) { [double]::Parse($c[8], [Globalization.CultureInfo]::InvariantCulture) } else { 0.0 }
		$f2 = if ($c[9]) { [double]::Parse($c[9], [Globalization.CultureInfo]::InvariantCulture) } else { 0.0 }
		$f = $f1 + $f2
		if ($f -le 0) { continue }
		if ($freq.ContainsKey($w)) { $freq[$w] += $f } else { $freq[$w] = $f }
	}
} finally {
	$reader.Close()
}

$top = $freq.GetEnumerator() | Sort-Object Value -Descending | Select-Object -First $Count
$lines = foreach ($e in $top) {
	# Hundredths of ln(frequency per million + 1).
	$score = [math]::Round([math]::Log($e.Value + 1) * 100)
	"$($e.Key)`t$score"
}
New-Item -ItemType Directory -Force (Split-Path $out) | Out-Null
[IO.File]::WriteAllText($out, (($lines -join "`n") + "`n"), (New-Object System.Text.UTF8Encoding($false)))

"entries: $($top.Count)"
"size: $((Get-Item $out).Length) bytes"

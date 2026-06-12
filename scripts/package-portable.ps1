Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$Root = Resolve-Path (Join-Path $PSScriptRoot "..")
$TauriConfigPath = Join-Path $Root "src-tauri\tauri.conf.json"
$TauriConfig = Get-Content -LiteralPath $TauriConfigPath -Raw -Encoding UTF8 | ConvertFrom-Json
$Version = [string]$TauriConfig.version

$ReleaseDir = Join-Path $Root "src-tauri\target\release"
$ExePath = Join-Path $ReleaseDir "moeplay.exe"
if (-not (Test-Path -LiteralPath $ExePath)) {
  throw "Release executable not found: $ExePath. Run npm.cmd run tauri -- build first."
}

$PortableDir = Join-Path $ReleaseDir "bundle\portable"
$StageDir = Join-Path $PortableDir "moeplay-$Version-x64-portable"
$ZipPath = Join-Path $PortableDir "moeplay_${Version}_x64-portable.zip"

New-Item -ItemType Directory -Force -Path $PortableDir | Out-Null
if (Test-Path -LiteralPath $StageDir) {
  Remove-Item -LiteralPath $StageDir -Recurse -Force
}
New-Item -ItemType Directory -Force -Path $StageDir | Out-Null

Copy-Item -LiteralPath $ExePath -Destination (Join-Path $StageDir "moeplay.exe") -Force

$Readme = @"
MoePlay portable package
Version: $Version

Run moeplay.exe directly. User data is stored in the normal application data
directory, so this package does not overwrite local libraries by itself.
"@
Set-Content -LiteralPath (Join-Path $StageDir "README.txt") -Value $Readme -Encoding UTF8

$Changelog = Join-Path $Root "CHANGELOG.md"
if (Test-Path -LiteralPath $Changelog) {
  Copy-Item -LiteralPath $Changelog -Destination (Join-Path $StageDir "CHANGELOG.md") -Force
}

if (Test-Path -LiteralPath $ZipPath) {
  Remove-Item -LiteralPath $ZipPath -Force
}
Compress-Archive -LiteralPath $StageDir -DestinationPath $ZipPath -CompressionLevel Optimal -Force

Write-Output $ZipPath

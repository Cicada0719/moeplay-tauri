param(
  [ValidateSet("Auto", "Required", "Disabled")]
  [string]$UpdaterMode = $(if ($env:UPDATER_RELEASE_MODE) { $env:UPDATER_RELEASE_MODE } else { "Auto" })
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$Root = Resolve-Path (Join-Path $PSScriptRoot "..")
Add-Type -AssemblyName System.IO.Compression.FileSystem
$TauriConfigPath = Join-Path $Root "src-tauri\tauri.conf.json"
$TauriConfig = Get-Content -LiteralPath $TauriConfigPath -Raw -Encoding UTF8 | ConvertFrom-Json
$Version = [string]$TauriConfig.version

$ReleaseDir = Join-Path $Root "src-tauri\target\release"
$BundleDir = Join-Path $ReleaseDir "bundle"
$ManifestPath = Join-Path $BundleDir "release-manifest.json"

function Get-LatestArtifact([string]$Directory, [string]$Pattern) {
  if (-not (Test-Path -LiteralPath $Directory)) {
    return $null
  }
  $Match = Get-ChildItem -LiteralPath $Directory -Filter $Pattern -File |
    Sort-Object LastWriteTime -Descending |
    Select-Object -First 1
  if ($null -eq $Match) {
    return $null
  }
  return $Match.FullName
}

function Get-Sha256([string]$Path) {
  $Command = Get-Command Get-FileHash -ErrorAction SilentlyContinue
  if ($null -ne $Command) {
    return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash
  }

  $Stream = [System.IO.File]::OpenRead($Path)
  try {
    $Sha = [System.Security.Cryptography.SHA256]::Create()
    try {
      $Bytes = $Sha.ComputeHash($Stream)
      return ([BitConverter]::ToString($Bytes) -replace "-", "").ToUpperInvariant()
    }
    finally {
      $Sha.Dispose()
    }
  }
  finally {
    $Stream.Dispose()
  }
}

$Expected = @(
  @{ Kind = "exe"; Path = (Join-Path $ReleaseDir "moeplay.exe"); MinBytes = 5MB },
  @{ Kind = "msi"; Path = (Get-LatestArtifact (Join-Path $BundleDir "msi") "*$Version*.msi"); MinBytes = 1MB },
  @{ Kind = "nsis"; Path = (Get-LatestArtifact (Join-Path $BundleDir "nsis") "*$Version*.exe"); MinBytes = 1MB },
  @{ Kind = "portable"; Path = (Join-Path $BundleDir "portable\moeplay_${Version}_x64-portable.zip"); MinBytes = 1MB },
  @{ Kind = "sbom"; Path = (Join-Path $BundleDir "sbom.cdx.json"); MinBytes = 1KB },
  @{ Kind = "build-metadata"; Path = (Join-Path $BundleDir "build-metadata.json"); MinBytes = 100 }
)

$Artifacts = @()
$UpdaterManifestPath = Join-Path $Root "latest.json"
$UpdaterVerifier = Join-Path $Root "scripts\verify-updater-artifacts.mjs"
$UpdaterStatus = "not-generated"
$UpdaterArguments = @($UpdaterVerifier)
if ($UpdaterMode -eq "Required") {
  $UpdaterArguments += @("--require", "--artifacts-dir", $ReleaseDir, $UpdaterManifestPath)
  $UpdaterStatus = "verified"
}
elseif ($UpdaterMode -eq "Disabled") {
  $UpdaterArguments += @("--expect-absent")
  $UpdaterStatus = "blocked-no-signing-secret"
}
elseif (Test-Path -LiteralPath $UpdaterManifestPath) {
  $UpdaterArguments += @("--artifacts-dir", $ReleaseDir, $UpdaterManifestPath)
  $UpdaterStatus = "verified"
}

& node @UpdaterArguments
if ($LASTEXITCODE -ne 0) {
  throw "Updater artifact verification failed in $UpdaterMode mode"
}

foreach ($Item in $Expected) {
  if (-not $Item.Path -or -not (Test-Path -LiteralPath $Item.Path)) {
    throw "Missing release artifact [$($Item.Kind)]: $($Item.Path)"
  }

  $File = Get-Item -LiteralPath $Item.Path
  if ($File.Length -lt $Item.MinBytes) {
    throw "Release artifact [$($Item.Kind)] is unexpectedly small: $($File.FullName) ($($File.Length) bytes)"
  }

  $Hash = Get-Sha256 $File.FullName
  $Artifacts += [pscustomobject]@{
    kind = $Item.Kind
    path = $File.FullName
    bytes = $File.Length
    sha256 = $Hash
    lastWriteTime = $File.LastWriteTime.ToString("o")
  }
}

if ($UpdaterMode -eq "Required") {
  $UpdaterFiles = @((Get-Item -LiteralPath $UpdaterManifestPath))
  $SignatureFiles = @(Get-ChildItem -LiteralPath $ReleaseDir -Recurse -File -Filter "*.sig")
  if ($SignatureFiles.Count -eq 0) {
    throw "Signed updater mode did not produce detached signature files"
  }
  $UpdaterFiles += $SignatureFiles
  foreach ($SignatureFile in $SignatureFiles) {
    $SignedArtifactPath = $SignatureFile.FullName.Substring(0, $SignatureFile.FullName.Length - 4)
    if (-not (Test-Path -LiteralPath $SignedArtifactPath)) {
      throw "Detached updater signature has no matching artifact: $($SignatureFile.FullName)"
    }
    $UpdaterFiles += Get-Item -LiteralPath $SignedArtifactPath
  }

  foreach ($File in @($UpdaterFiles | Sort-Object FullName -Unique)) {
    if ($Artifacts.path -contains $File.FullName) {
      continue
    }
    $Artifacts += [pscustomobject]@{
      kind = $(if ($File.Name -eq "latest.json") { "updater-manifest" } elseif ($File.Extension -eq ".sig") { "updater-signature" } else { "updater-artifact" })
      path = $File.FullName
      bytes = $File.Length
      sha256 = Get-Sha256 $File.FullName
      lastWriteTime = $File.LastWriteTime.ToString("o")
    }
  }
}
elseif ($UpdaterMode -eq "Disabled") {
  $UnexpectedSignatures = @(Get-ChildItem -LiteralPath $ReleaseDir -Recurse -File -Filter "*.sig" -ErrorAction SilentlyContinue)
  if ($UnexpectedSignatures.Count -gt 0) {
    throw "Unsigned/degraded mode produced updater signatures unexpectedly: $($UnexpectedSignatures.FullName -join ', ')"
  }
}

$Portable = $Artifacts | Where-Object { $_.kind -eq "portable" } | Select-Object -First 1
if ($null -eq $Portable) {
  throw "Portable artifact was not collected"
}
$RequiredEntries = @("moeplay.exe", "README.txt", "CHANGELOG.md")
$Zip = [System.IO.Compression.ZipFile]::OpenRead($Portable.path)
try {
  $EntryNames = @($Zip.Entries | ForEach-Object { Split-Path -Leaf $_.FullName })
  foreach ($Required in $RequiredEntries) {
    if ($EntryNames -notcontains $Required) {
      throw "Portable zip is missing required entry: $Required"
    }
  }
}
finally {
  $Zip.Dispose()
}

$SbomPath = Join-Path $BundleDir "sbom.cdx.json"
$Sbom = Get-Content -LiteralPath $SbomPath -Raw -Encoding UTF8 | ConvertFrom-Json
if ($Sbom.bomFormat -ne "CycloneDX" -or $Sbom.specVersion -ne "1.5") {
  throw "SBOM is not a CycloneDX 1.5 document"
}
if ([string]$Sbom.metadata.component.version -ne $Version) {
  throw "SBOM version does not match release version"
}
if (@($Sbom.components).Count -lt 10) {
  throw "SBOM contains unexpectedly few components"
}

$BuildMetadataPath = Join-Path $BundleDir "build-metadata.json"
$BuildMetadata = Get-Content -LiteralPath $BuildMetadataPath -Raw -Encoding UTF8 | ConvertFrom-Json
if ([string]$BuildMetadata.version -ne $Version) {
  throw "Build metadata version does not match release version"
}
if (-not $BuildMetadata.commit -or -not $BuildMetadata.toolchain.rustc -or -not $BuildMetadata.toolchain.node) {
  throw "Build metadata is missing commit or toolchain evidence"
}
if ($env:CI -and $BuildMetadata.dirty -eq $true) {
  throw "CI release build metadata reports a dirty worktree"
}

$Manifest = [pscustomobject]@{
  productName = [string]$TauriConfig.productName
  version = $Version
  generatedAt = (Get-Date).ToUniversalTime().ToString("o")
  updater = [pscustomobject]@{
    mode = $UpdaterMode.ToLowerInvariant()
    status = $UpdaterStatus
    manifest = $(if (Test-Path -LiteralPath $UpdaterManifestPath) { $UpdaterManifestPath } else { $null })
  }
  artifacts = $Artifacts
}

New-Item -ItemType Directory -Force -Path $BundleDir | Out-Null
$Manifest | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $ManifestPath -Encoding UTF8
Write-Output $ManifestPath

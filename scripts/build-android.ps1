param(
    [ValidateSet('Unsigned','Release','Compat')][string]$Channel = 'Unsigned',
    [string]$TargetDirectory = $env:CARGO_TARGET_DIR,
    [string]$OutputDirectory,
    [switch]$SkipRust,
    [switch]$SkipFrontend
)
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path $PSScriptRoot -Parent
Set-Location -LiteralPath $projectRoot
function Invoke-Checked([string]$Program, [string[]]$Arguments) {
    & $Program @Arguments
    if ($LASTEXITCODE -ne 0) { throw "$Program failed with exit code $LASTEXITCODE" }
}
if (!$env:ANDROID_HOME) { $env:ANDROID_HOME = Join-Path $env:LOCALAPPDATA 'Android/Sdk' }
if (!$env:NDK_HOME) {
    $ndk = Get-ChildItem (Join-Path $env:ANDROID_HOME 'ndk') -Directory | Sort-Object Name -Descending | Select-Object -First 1
    if (!$ndk) { throw 'Install Android NDK or set NDK_HOME' }
    $env:NDK_HOME = $ndk.FullName
}
if (!$env:JAVA_HOME) { throw 'Set JAVA_HOME to JDK 17' }
if (!$env:LIBCLANG_PATH) { throw 'Set LIBCLANG_PATH to the folder containing libclang.dll (pip install libclang)' }
if (!$TargetDirectory) { $TargetDirectory = Join-Path ([IO.Path]::GetTempPath()) 'moeplay-android-target' }
if ($TargetDirectory -match '[^\x00-\x7F]') { throw 'CARGO_TARGET_DIR must use an ASCII path for the Windows NDK linker' }
$env:CARGO_TARGET_DIR = $TargetDirectory
$env:PATH = "$(Join-Path $env:JAVA_HOME 'bin');$env:PATH"
$toolchain = Join-Path $env:NDK_HOME 'toolchains/llvm/prebuilt/windows-x86_64'
$clangVersion = Get-ChildItem (Join-Path $toolchain 'lib/clang') -Directory | Sort-Object Name -Descending | Select-Object -First 1
$sysroot = (Join-Path $toolchain 'sysroot').Replace('\','/')
$include = (Join-Path $clangVersion.FullName 'include').Replace('\','/')
$env:BINDGEN_EXTRA_CLANG_ARGS_AARCH64_LINUX_ANDROID = "--sysroot=$sysroot -I$include"
$env:CC_aarch64_linux_android = Join-Path $toolchain 'bin/clang.exe'
$env:CXX_aarch64_linux_android = Join-Path $toolchain 'bin/clang++.exe'
$env:AR_aarch64_linux_android = Join-Path $toolchain 'bin/llvm-ar.exe'
$env:CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER = Join-Path $toolchain 'bin/aarch64-linux-android24-clang.cmd'
$version = (Get-Content package.json -Raw | ConvertFrom-Json).version
$parts = $version.Split('.')
$versionCode = [int]$parts[0] * 1000000 + [int]$parts[1] * 1000 + [int]$parts[2]
if (!$SkipFrontend) { Invoke-Checked 'node' @('node_modules/vite/bin/vite.js','build') }
if (!$SkipRust) { Invoke-Checked 'cargo' @('build','--package','moeplay','--manifest-path','src-tauri/Cargo.toml','--target','aarch64-linux-android','--release','--features','tauri/custom-protocol','--lib') }
$jni = Join-Path $projectRoot 'src-tauri/gen/android/app/src/main/jniLibs/arm64-v8a'
New-Item -ItemType Directory -Path $jni -Force | Out-Null
Copy-Item -LiteralPath (Join-Path $TargetDirectory 'aarch64-linux-android/release/libmoeplay_lib.so') -Destination $jni -Force
Invoke-Checked (Join-Path $toolchain 'bin/llvm-strip.exe') @('--strip-unneeded',(Join-Path $jni 'libmoeplay_lib.so'))
$buildType = if ($Channel -eq 'Compat') { 'Debug' } else { 'Release' }
Push-Location 'src-tauri/gen/android'
try {
    Invoke-Checked '.\gradlew.bat' @("assembleArm64$buildType",'-x',"rustBuildArm64$buildType", "-PmoeplayVersionName=$version", "-PmoeplayVersionCode=$versionCode",'--no-daemon')
} finally { Pop-Location }
$output = if ($OutputDirectory) { [IO.Path]::GetFullPath($OutputDirectory) } else { Join-Path $projectRoot "artifacts/$version" }
New-Item -ItemType Directory -Path $output -Force | Out-Null
$type = $buildType.ToLowerInvariant()
$apk = Get-ChildItem "src-tauri/gen/android/app/build/outputs/apk/arm64/$type/*.apk" | Select-Object -First 1
if (!$apk) { throw 'ARM64 APK was not generated' }
$destination = Join-Path $output "MoeGame_${version}_arm64-$($Channel.ToLowerInvariant()).apk"
if (Test-Path -LiteralPath $destination) { throw "Artifact already exists: $destination. Use a new staging directory before rebuilding." }
if ($Channel -eq 'Release') {
    foreach ($name in @('ANDROID_KEYSTORE_PATH','ANDROID_STORE_PASSWORD','ANDROID_KEY_ALIAS','ANDROID_KEY_PASSWORD')) {
        if (![Environment]::GetEnvironmentVariable($name)) { throw "Set $name outside the repository" }
    }
    $buildTools = Get-ChildItem (Join-Path $env:ANDROID_HOME 'build-tools') -Directory | Sort-Object Name -Descending | Select-Object -First 1
    Invoke-Checked (Join-Path $buildTools.FullName 'apksigner.bat') @('sign','--ks',$env:ANDROID_KEYSTORE_PATH,'--ks-key-alias',$env:ANDROID_KEY_ALIAS,'--ks-pass','env:ANDROID_STORE_PASSWORD','--key-pass','env:ANDROID_KEY_PASSWORD','--out',$destination,$apk.FullName)
    Invoke-Checked (Join-Path $buildTools.FullName 'apksigner.bat') @('verify','--verbose','--print-certs',$destination)
} else { Copy-Item -LiteralPath $apk.FullName -Destination $destination }
Write-Output "Built $destination (versionCode $versionCode)"

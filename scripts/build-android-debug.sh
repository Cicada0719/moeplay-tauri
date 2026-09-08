#!/usr/bin/env bash
# build-android-debug.sh — 萌游 Android debug APK 一键构建（Windows / Git Bash）
#
# 用法：在仓库根目录执行  bash scripts/build-android-debug.sh
# 产物：src-tauri/gen/android/app/build/outputs/apk/arm64/debug/app-arm64-debug.apk
#
# 前置条件：
#   - Android SDK（含 NDK、build-tools、platforms），默认取 %LOCALAPPDATA%\Android\Sdk
#   - JDK 17（使用 JAVA_HOME 指向本机安装目录）
#   - Rust 工具链 + aarch64-linux-android target（rustup target add aarch64-linux-android）
#   - libclang（bindgen 生成 rquickjs Android 绑定用）：
#       pip install libclang  →  然后设 LIBCLANG_PATH 指向 clang\native 目录
#
# 已知坑（脚本已内置规避）：
#   1. 工作区路径含中文 → ld.lld 无法打开中间产物，CARGO_TARGET_DIR 须为纯 ASCII 路径。
#   2. Windows 无符号链接权限 → tauri CLI 拷 .so 进 jniLibs 失败，改为手动 cp 后直接跑 gradle。
#   3. AGP 拒绝非 ASCII 项目路径 → gen/android/gradle.properties 已加 android.overridePathCheck=true。
#   4. tauri gradle 插件的 rustBuild* 任务会重跑 cargo 且依赖 node.bat，构建时排除（-x）。

set -euo pipefail
cd "$(dirname "$0")/.."

# ── 环境（按需覆盖）───────────────────────────────────────────────
export ANDROID_HOME="${ANDROID_HOME:-$LOCALAPPDATA/Android/Sdk}"
export NDK_HOME="${NDK_HOME:-$ANDROID_HOME/ndk/28.0.13004108}"
: "${JAVA_HOME:?Set JAVA_HOME to JDK 17}"
export ProgramData="${ProgramData:-C:\\ProgramData}"
export PATH="$HOME/.cargo/bin:$PATH"

# rquickjs-sys 无 Android 预生成绑定，需 bindgen 现场生成（见 src-tauri/Cargo.toml）
NDK_POSIX="$(cygpath "$NDK_HOME")"
: "${LIBCLANG_PATH:?需要 LIBCLANG_PATH 指向 libclang.dll 所在目录（pip install libclang 后在其 clang/native 下）}"
export BINDGEN_EXTRA_CLANG_ARGS_AARCH64_LINUX_ANDROID="--sysroot=$NDK_POSIX/toolchains/llvm/prebuilt/windows-x86_64/sysroot -I$NDK_POSIX/toolchains/llvm/prebuilt/windows-x86_64/lib/clang/19/include"

# 交叉编译器 / 链接器（cc-rs 与 rustc 都需要显式指向 NDK 工具链）：
#   - CC_*/CXX_*/AR_*：cc-rs 编译 C 依赖（找不到时报 failed to find tool "clang.exe"）
#   - CARGO_TARGET_*_LINKER：rustc 链接（找不到时报 linker `cc` not found）；
#     NDK 的 target 前缀 .cmd 已内置 --target=aarch64-linux-android24（与 minSdk 一致）
NDK_TOOLCHAIN_BIN="$NDK_HOME/toolchains/llvm/prebuilt/windows-x86_64/bin"
export CC_aarch64_linux_android="${CC_aarch64_linux_android:-$NDK_TOOLCHAIN_BIN\\clang.exe}"
export CXX_aarch64_linux_android="${CXX_aarch64_linux_android:-$NDK_TOOLCHAIN_BIN\\clang++.exe}"
export AR_aarch64_linux_android="${AR_aarch64_linux_android:-$NDK_TOOLCHAIN_BIN\\llvm-ar.exe}"
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="${CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER:-$NDK_TOOLCHAIN_BIN\\aarch64-linux-android24-clang.cmd}"

# 坑 1：纯 ASCII 构建目录（可用 CARGO_TARGET_DIR 覆盖）
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$TEMP/moeplay-android-target}"

# ── 1. Rust → libmoeplay_lib.so ─────────────────────────────────
echo "==> cargo build (aarch64-linux-android, debug)"
cargo build --package moeplay --manifest-path src-tauri/Cargo.toml \
  --target aarch64-linux-android --features tauri/custom-protocol --lib

# ── 2. 复制 + 裁剪符号（坑 2）────────────────────────────────────
JNI_DIR=src-tauri/gen/android/app/src/main/jniLibs/arm64-v8a
mkdir -p "$JNI_DIR"
cp -f "$(cygpath "$CARGO_TARGET_DIR")/aarch64-linux-android/debug/libmoeplay_lib.so" "$JNI_DIR/"
"$NDK_POSIX/toolchains/llvm/prebuilt/windows-x86_64/bin/llvm-strip.exe" \
  --strip-unneeded "$JNI_DIR/libmoeplay_lib.so" || true

# ── 3. Gradle 打包（排除 rustBuild*，坑 4）───────────────────────
echo "==> gradle assembleDebug"
cd src-tauri/gen/android
./gradlew assembleDebug \
  -x rustBuildArm64Debug -x rustBuildArmDebug \
  -x rustBuildX86Debug -x rustBuildX86_64Debug -x rustBuildUniversalDebug

echo "==> 完成：src-tauri/gen/android/app/build/outputs/apk/arm64/debug/app-arm64-debug.apk"

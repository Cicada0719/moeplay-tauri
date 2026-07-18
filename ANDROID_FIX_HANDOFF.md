# MoePlay Android 闪退修复交接文档

> 更新时间：2026-07-17 15:00
> 工作区：`D:\我的文件\桌面备份\hermes\moeplay-tauri`
> 包名：`com.moeplay.app`　版本：0.13.9
> 状态：**✅ 修复已写入并在真机（OnePlus PKR110）验证通过：进程稳定存活、无 panic、主界面正常渲染**
>
> 验证证据：安装后 `pidof com.moeplay.app` 在 8s/13s 均返回稳定 PID；logcat 无 `PANIC`/`android context was not initialized`/`SIGABRT`；截图 `test-results/android-fixed.png` 显示番剧页完整 UI。
>
> 遗留小问题（非崩溃）：番剧页"规则"标签调用 `anime_github_rules_index` 被权限层拒绝，提示需要 `allow-anime-github-rules-index`，需在 `src-tauri/capabilities` 给该命令补 allow 权限。

---

## 1. 闪退根因（已确诊，勿再重复排查）

**调用链**（来自真机 logcat 崩溃日志）：

```
moeplay_lib::run
  → secret_store::SecretStore::new            （src-tauri/src/lib.rs 的 .manage(...) 链）
    → android_native_keyring_store::Store::new
      → ndk_context::android_context()
        → PANIC: "android context was not initialized"
          （ndk-context-0.1.1/src/lib.rs:72）
→ Rust abort → SIGABRT → 进程启动约 200ms 内死亡
```

**为什么**：tauri 2.11.2 / tao 0.35.3 / wry 0.55.1 这代框架**从不初始化 `ndk-context` crate 的全局上下文**。tao 把 JVM/activity 存在自己的注册表（`tao::platform_impl::android::ndk_glue::CONTEXTS`，经 `onActivityCreate` 写入），公开读取口是 `main_android_context()`。而 `android-native-keyring-store` 在构造时必须读 `ndk_context` 全局，为空即 panic。桌面端不需要这个上下文，所以只有 Android 闪退。

关键日志特征：`RustStdoutStderr` 中出现 `PANIC, location: ...ndk-context-0.1.1\src\lib.rs:72, payload: android context was not initialized`，且 logcat crash 缓冲区有 `SIGABRT` + `moeplay_lib::stop_unwind` backtrace。

## 2. 已应用的修复（已写入工作区，未提交 git）

### 2.1 `src-tauri/Cargo.toml`

在 `[target.'cfg(target_os = "android")'.dependencies]` 下新增：

```toml
# Must stay on the same release as the copy pulled in by
# android-native-keyring-store so both crates share one global context slot.
ndk-context = "0.1.1"
```

⚠️ 版本必须与 `Cargo.lock` 中 `android-native-keyring-store` 传递引入的 ndk-context 一致（0.1.1），否则两处 static 不共享，等于白修。

### 2.2 `src-tauri/src/lib.rs`

新增函数（在 `configure_android_paths()` 之后）：

```rust
#[cfg(target_os = "android")]
fn init_android_ndk_context() {
    use tauri::tao::platform::android::prelude::main_android_context;

    match main_android_context() {
        Some(context) => {
            unsafe {
                ndk_context::initialize_android_context(
                    context.java_vm,
                    context.context_jobject,
                );
            }
            crash_log("init_android_ndk_context() done");
        }
        None => crash_log("init_android_ndk_context() skipped: no android context"),
    }
}
```

并在 `run()` 开头（`configure_android_paths();` 之后、`crash_log("run() START");` 之前）调用：

```rust
    #[cfg(target_os = "android")]
    init_android_ndk_context();
```

时序依据：`onActivityCreate`（JNI）先把上下文写入 tao 的 CONTEXTS，之后 `create` 才 spawn 线程跑 `main()` → `run()`，所以 `run()` 里 `main_android_context()` 一定有值。

`keyring_core::set_default_store` 经核实可重复调用（RwLock 覆盖，无 OnceCell panic），多处 `SecretStore::new()` 安全。

## 3. 当前构建状态

- Rust 库已于 **14:31** 重新编译（含修复），位于 `src-tauri/gen/android/app/src/main/jniLibs/arm64-v8a/libmoeplay_lib.so`。
- **新 APK 已出包（14:45:26）**：`src-tauri/gen/android/app/build/outputs/apk/arm64/debug/app-arm64-debug.apk`（约 433 MB，debug 包）。
- 该 APK 包含上述修复，**可直接安装验证，无需重新构建**。

## 4. 环境信息（本机实测）

| 工具 | 路径 |
|---|---|
| adb | `C:\Users\sgy\AppData\Local\Android\Sdk\platform-tools\adb.exe`（本会话新装，PATH 里没有） |
| Java 17 | `C:\Program Files\Microsoft\jdk-17.0.19.10-hotspot`（设 `JAVA_HOME`） |
| Android SDK | `C:\Users\sgy\AppData\Local\Android\Sdk`（设 `ANDROID_HOME`） |
| NDK | `C:\Users\sgy\AppData\Local\Android\Sdk\ndk\28.0.13004108`（设 `ANDROID_NDK_HOME`/`NDK_HOME`） |
| cargo/rustup | `C:\Users\sgy\.cargo\bin`（rust target `aarch64-linux-android` 已装） |
| 测试机 | OnePlus PKR110，arm64，序列号 `491c0696`（**当前已断开，需重新插线/开 USB 调试**） |

### 坑（实测踩过）

1. **裸 Git Bash 跑 `cargo check/build --target aarch64-linux-android` 会在 `vswhom-sys` 失败**（host 构建脚本调 cl.exe，缺 MSVC INCLUDE/LIB）。正经构建用 `npm run tauri android build -- --debug --target aarch64`；若还缺 MSVC 环境，先调 `D:\my work\VC\Auxiliary\Build\vcvars64.bat`。
2. **本机无系统级 Node**，npm 是 shim：`C:\Users\sgy\AppData\Roaming\kimi-desktop\daimon-share\daimon\command-process-owner\bin\npm.cmd`，依赖环境变量 `KIMI_DESKTOP_RUNTIME_NODE` / `KIMI_DESKTOP_PROCESS_RUNNER`。gradle 的 `rustBuildArm64Debug` 任务要 `npm.bat`，直接跑 gradle 会报 "A problem occurred starting process 'command 'npm.bat''"。
3. **Git Bash 的 `cmd //c` 处理不了含中文路径的 .bat**（cmd 按 GBK 解析 UTF-8 的 bat 会炸）。方案：用 Python 生成 **带 UTF-8 BOM 的 .ps1**，再用 `C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe -NoProfile -ExecutionPolicy Bypass -File ...` 执行。现成的脚本在工作区 `run-build.ps1`。
4. 只改了 Rust 时，可跳过 Rust 重编译快速出包：`gradlew.bat assembleArm64Debug -x rustBuildArm64Debug`（前提是 jniLibs 里的 .so 是新的）。

## 5. 剩余步骤（约 2 分钟）

手机重新连接并确认 `adb devices` 可见后，依次执行（Git Bash）：

```bash
ADB="/c/Users/sgy/AppData/Local/Android/Sdk/platform-tools/adb.exe"

# 1. 安装（覆盖安装，保留数据）
"$ADB" install -r "D:\我的文件\桌面备份\hermes\moeplay-tauri\src-tauri\gen\android\app\build\outputs\apk\arm64\debug\app-arm64-debug.apk"

# 2. 清日志 + 冷启动
"$ADB" logcat -b all -c
"$ADB" shell am force-stop com.moeplay.app
"$ADB" shell monkey -p com.moeplay.app -c android.intent.category.LAUNCHER 1
sleep 8

# 3. 进程存活（有数字输出 = 没闪退）
"$ADB" shell pidof com.moeplay.app

# 4. 日志核对：不应再有 "android context was not initialized" / PANIC / SIGABRT；
#    应看到 "MoeGame v0.13.9 starting"
"$ADB" logcat -d -b all | grep -E "PANIC|android context|SIGABRT|MoeGame v"

# 5. 截图取证
"$ADB" exec-out screencap -p > "D:\我的文件\桌面备份\hermes\moeplay-tauri\test-results\android-fixed.png"
```

**验收标准**：`pidof` 有 PID 输出且 8 秒后仍存活；logcat 无新 panic；截图显示应用主界面正常渲染。

## 6. 若出现新的崩溃

提取完整 panic 块再修：

```bash
"$ADB" logcat -d -b all | grep "RustStdoutStderr"
# 或崩溃缓冲区
"$ADB" logcat -d -b crash
```

把 backtrace 中含 `moeplay_lib` 的帧贴出来定位。修复后增量重出包：重编 Rust（`npm run tauri android build -- --debug --target aarch64`，或确认 .so 更新后 `gradlew.bat assembleArm64Debug -x rustBuildArm64Debug`），再回到第 5 节。

## 7. 纪律

- 不要 `git reset --hard` / `git clean` / 覆盖式 checkout（工作区有大量未提交改造，git 可能报 dubious ownership，用 `git -c safe.directory='D:/我的文件/桌面备份/hermes/moeplay-tauri' ...`）。
- 背景文档：`ANDROID_HANDOFF.md`（部分内容已过时：gen/android 已生成、APK 已能编译）。

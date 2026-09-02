package com.moeplay.handheld

import android.app.Activity
import android.content.ActivityNotFoundException
import android.content.ComponentName
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.os.Environment
import android.os.storage.StorageManager
import android.provider.Settings
import android.webkit.MimeTypeMap
import android.view.View
import android.view.WindowInsets
import android.view.WindowInsetsController
import androidx.core.content.FileProvider
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSArray
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import java.io.File
import org.json.JSONObject

@InvokeArg
class LaunchGameArgs {
    var packageName: String = ""
    lateinit var romPath: String
}

@InvokeArg
class SystemBarsArgs {
    var immersive: Boolean = true
}

/**
 * 安卓掌机桥：模拟器探测、ROM 启动（Intent）、全盘文件访问授权。
 *
 * ES-DE 在 Android 上的等价能力：列出已安装模拟器 → 以 ACTION_VIEW +
 * FileProvider content:// URI 把 ROM 交给模拟器（RetroArch 额外读取 "ROM" extra）。
 */
@TauriPlugin
class HandheldPlugin(private val activity: Activity) : Plugin(activity) {

    companion object {
        /** 常见模拟器包名 → (显示名, 适用平台)。按 ES-DE / Daijishō 常见装机清单整理。 */
        private val KNOWN_EMULATORS: List<Triple<String, String, List<String>>> = listOf(
            Triple("com.retroarch.aarch64", "RetroArch", listOf("nes", "snes", "n64", "gb", "gbc", "gba", "nds", "ps1", "psp", "arcade", "md", "saturn", "dreamcast")),
            Triple("com.retroarch", "RetroArch (32-bit)", listOf("nes", "snes", "n64", "gb", "gbc", "gba", "nds", "ps1", "psp", "arcade", "md")),
            Triple("com.swordfish.lemuroid", "Lemuroid", listOf("nes", "snes", "n64", "gb", "gbc", "gba", "nds", "ps1", "psp", "md")),
            Triple("org.ppsspp.ppsspp", "PPSSPP", listOf("psp")),
            Triple("org.ppsspp.ppssppgold", "PPSSPP Gold", listOf("psp")),
            Triple("org.dolphinemu.dolphinemu", "Dolphin", listOf("gamecube", "wii")),
            Triple("org.dolphinemu.mmjr", "Dolphin MMJR", listOf("gamecube", "wii")),
            Triple("xyz.aethersx2.android", "AetherSX2", listOf("ps2")),
            Triple("com.github.stenzek.duckstation", "DuckStation", listOf("ps1")),
            Triple("com.epsxe.ePSXe", "ePSXe", listOf("ps1")),
            Triple("com.emulator.fpse", "FPse", listOf("ps1")),
            Triple("org.citra.citra_emu", "Citra", listOf("3ds")),
            Triple("org.citra.emu", "Citra (MMJ)", listOf("3ds")),
            Triple("org.citra.citra_canary", "Citra Canary", listOf("3ds")),
            Triple("io.github.lime3ds.android", "Lime3DS", listOf("3ds")),
            Triple("com.dsemu.drastic", "DraStic", listOf("nds")),
            Triple("me.magnum.melonds", "melonDS", listOf("nds")),
            Triple("org.mupen64plusae.v3.fzurita.pro", "Mupen64Plus FZ Pro", listOf("n64")),
            Triple("org.mupen64plusae.v3.fzurita", "Mupen64Plus FZ", listOf("n64")),
            Triple("com.onscripter.plus", "ONScripter Plus", listOf("onscripter")),
            Triple("it.dbtecno.pizzaboygba", "Pizza Boy GBA", listOf("gba")),
            Triple("it.dbtecno.pizzaboygbapro", "Pizza Boy GBA Pro", listOf("gba")),
            Triple("com.fastemulator.gba", "My Boy!", listOf("gba")),
            Triple("com.fastemulator.gbc", "My OldBoy!", listOf("gb", "gbc")),
            Triple("com.flycast.emulator", "Flycast", listOf("dreamcast", "naomi", "atomiswave")),
            Triple("io.recompiled.redream", "Redream", listOf("dreamcast")),
            Triple("org.vita3k.emulator", "Vita3K", listOf("psvita")),
            Triple("org.yuzu.yuzu_emu", "Yuzu", listOf("switch")),
            Triple("skyline.emu", "Skyline", listOf("switch")),
            Triple("com.nostalgiaemulators.nes1", "Nostalgia.NES", listOf("nes")),
            Triple("com.nostalgiaemulators.snes1", "Nostalgia.SNES", listOf("snes")),
            Triple("com.nostalgiaemulators.n64", "Nostalgia.N64", listOf("n64")),
        )
    }

    /**
     * 通过 StorageManager 枚举存储卷并探测常见 ROM 目录。
     * 部分设备（如 MANGMI）即使授予「所有文件访问」也不允许应用列举 /storage 根目录，
     * 但 StorageManager 能直接给出各卷路径，且卷内目录可以正常读取。
     */
    @Command
    fun discoverRomRoots(invoke: Invoke) {
        try {
            val sm = activity.getSystemService(StorageManager::class.java)
            val subdirs = listOf("rom", "roms", "ROMs", "Roms", "Emulation/roms")
            val result = JSArray()
            val seen = HashSet<String>()
            for (volume in sm.storageVolumes) {
                if (volume.state != Environment.MEDIA_MOUNTED) continue
                @Suppress("DEPRECATION")
                val dir = volume.directory ?: continue
                val desc = if (volume.isRemovable) {
                    // 部分设备的卷描述是文件系统卷标，可能含无法显示的字符，统一回退为「SD 卡」
                    val raw = volume.getDescription(activity) ?: ""
                    if (raw.isBlank() || raw.contains('?') || raw.contains('�')) "SD 卡" else raw
                } else {
                    "内置存储"
                }
                for (sub in subdirs) {
                    val candidate = File(dir, sub)
                    if (!candidate.isDirectory) continue
                    val path = candidate.absolutePath
                    if (seen.add(path)) {
                        result.put(JSObject().apply {
                            put("path", path)
                            put("label", "$desc $sub")
                        })
                    }
                }
            }
            invoke.resolve(JSObject().apply { put("roots", result) })
        } catch (error: Exception) {
            invoke.reject(error.message ?: "Failed to discover ROM roots")
        }
    }

    @Command
    fun listEmulators(invoke: Invoke) {        try {
            val pm = activity.packageManager
            val result = JSArray()
            for ((packageName, label, platforms) in KNOWN_EMULATORS) {
                val version = try {
                    @Suppress("DEPRECATION")
                    pm.getPackageInfo(packageName, 0).versionName ?: ""
                } catch (e: PackageManager.NameNotFoundException) {
                    continue
                }
                val platformArray = JSArray()
                for (p in platforms) platformArray.put(p)
                result.put(JSObject().apply {
                    put("packageName", packageName)
                    put("label", label)
                    put("versionName", version)
                    put("platforms", platformArray)
                })
            }
            invoke.resolve(JSObject().apply { put("emulators", result) })
        } catch (error: Exception) {
            invoke.reject(error.message ?: "Failed to list emulators")
        }
    }

    @Command
    fun launchGame(invoke: Invoke) {
        try {
            val args = invoke.parseArgs(LaunchGameArgs::class.java)
            val romFile = File(args.romPath)
            if (!romFile.exists()) {
                invoke.reject("ROM 文件不存在: ${args.romPath}")
                return
            }

            // 有序候选链（Rust 侧按平台解析）：按序 startActivity 直到成功。
            val candidates = JSONObject(invoke.getRawArgs()).optJSONArray("candidates")
            if (candidates != null && candidates.length() > 0) {
                val errors = mutableListOf<String>()
                for (i in 0 until candidates.length()) {
                    val candidate = candidates.optJSONObject(i) ?: continue
                    val mode = candidate.optString("mode")
                    val pkg = candidate.optString("package")
                    if (pkg.isEmpty()) continue
                    try {
                        when (mode) {
                            "retroarch" -> {
                                val activityClass = candidate.optString("activity")
                                    .ifEmpty { "com.retroarch.browser.retroactivity.RetroActivityFuture" }
                                val core = candidate.optString("core")
                                val intent = Intent().apply {
                                    component = ComponentName(pkg, activityClass)
                                    putExtra("ROM", args.romPath)
                                    putExtra("LIBRETRO", core)
                                    putExtra(
                                        "CONFIGFILE",
                                        "/storage/emulated/0/Android/data/$pkg/files/retroarch.cfg"
                                    )
                                    addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
                                }
                                activity.startActivity(intent)
                                invoke.resolve(JSObject().apply {
                                    put("launched", true)
                                    put("strategy", "retroarch:$core")
                                })
                                return
                            }
                            "view" -> {
                                activity.startActivity(buildViewIntent(pkg, romFile, args.romPath))
                                invoke.resolve(JSObject().apply {
                                    put("launched", true)
                                    put("strategy", "view:$pkg")
                                })
                                return
                            }
                            "menu" -> {
                                val menuIntent =
                                    activity.packageManager.getLaunchIntentForPackage(pkg)
                                if (menuIntent == null) {
                                    errors.add("$pkg: 未安装")
                                    continue
                                }
                                menuIntent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
                                menuIntent.putExtra("ROM", args.romPath)
                                menuIntent.putExtra("rom", args.romPath)
                                menuIntent.putExtra("PATH", args.romPath)
                                activity.startActivity(menuIntent)
                                invoke.resolve(JSObject().apply {
                                    put("launched", true)
                                    put("strategy", "menu:$pkg")
                                })
                                return
                            }
                            else -> errors.add("$pkg: 未知模式 $mode")
                        }
                    } catch (e: Exception) {
                        errors.add("$pkg/$mode: ${e.message ?: e.javaClass.simpleName}")
                    }
                }
                invoke.reject("所有启动方式均失败 — ${errors.joinToString("; ")}")
                return
            }

            // 旧式两级策略（无候选链时兼容）：view → launcher。
            try {
                activity.startActivity(buildViewIntent(args.packageName, romFile, args.romPath))
                invoke.resolve(JSObject().apply {
                    put("launched", true)
                    put("strategy", "view-intent")
                })
                return
            } catch (e: ActivityNotFoundException) {
                // fall through to launcher intent
            }

            // 策略 2：唤起模拟器主界面，并附带 ROM extra。
            // RetroArch 正是通过主 activity 的 "ROM"/"LIBRETRO"/"CONFIGFILE" extra 接游戏；
            // 其余模拟器忽略多余 extra，至少落在主界面可手动选择。
            val launchIntent = activity.packageManager.getLaunchIntentForPackage(args.packageName)
            if (launchIntent != null) {
                launchIntent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
                launchIntent.putExtra("ROM", args.romPath)
                launchIntent.putExtra("rom", args.romPath)
                launchIntent.putExtra("PATH", args.romPath)
                activity.startActivity(launchIntent)
                invoke.resolve(JSObject().apply {
                    put("launched", true)
                    put("strategy", "launcher-intent")
                })
            } else {
                invoke.reject("未安装模拟器: ${args.packageName}")
            }
        } catch (error: Exception) {
            invoke.reject(error.message ?: "Failed to launch game")
        }
    }

    /** ACTION_VIEW + FileProvider content URI（多数模拟器接受该形式直进游戏）。 */
    private fun buildViewIntent(pkg: String, romFile: File, romPath: String): Intent {
        val authority = activity.packageName + ".fileprovider"
        val romUri = FileProvider.getUriForFile(activity, authority, romFile)
        val extension = romFile.extension.lowercase()
        val mime = MimeTypeMap.getSingleton().getMimeTypeFromExtension(extension)
            ?: "application/octet-stream"
        return Intent(Intent.ACTION_VIEW).apply {
            setPackage(pkg)
            setDataAndType(romUri, mime)
            addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
            addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
            putExtra("ROM", romPath)
            putExtra("rom", romPath)
            putExtra("PATH", romPath)
        }
    }

    @Command
    fun hasAllFilesAccess(invoke: Invoke) {
        invoke.resolve(JSObject().apply { put("granted", allFilesGranted()) })
    }

    @Command
    fun requestAllFilesAccess(invoke: Invoke) {
        try {
            if (!allFilesGranted() && Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
                val intent = Intent(Settings.ACTION_MANAGE_APP_ALL_FILES_ACCESS_PERMISSION).apply {
                    data = Uri.parse("package:${activity.packageName}")
                    addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
                }
                activity.startActivity(intent)
            }
            // 授权发生在系统设置页，当前状态立即返回；前端应稍后重新查询。
            invoke.resolve(JSObject().apply { put("granted", allFilesGranted()) })
        } catch (error: Exception) {
            invoke.reject(error.message ?: "Failed to request all-files access")
        }
    }

    /**
     * 掌机显示模式：隐藏或恢复 Android 状态栏与底部导航栏。
     *
     * 使用 transient immersive 行为，用户从屏幕边缘滑动时仍可临时呼出系统栏，
     * 但不会让三键导航栏永久占用游戏界面高度。旧版 Android 使用兼容 flags。
     */
    @Command
    fun setSystemBars(invoke: Invoke) {
        try {
            val args = invoke.parseArgs(SystemBarsArgs::class.java)
            val immersive = args.immersive
            activity.runOnUiThread {
                val decor = activity.window.decorView
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
                    activity.window.setDecorFitsSystemWindows(!immersive)
                    activity.window.insetsController?.let { controller ->
                        controller.systemBarsBehavior =
                            WindowInsetsController.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE
                        if (immersive) {
                            controller.hide(WindowInsets.Type.systemBars())
                        } else {
                            controller.show(WindowInsets.Type.systemBars())
                        }
                    }
                } else {
                    var flags = View.SYSTEM_UI_FLAG_LAYOUT_STABLE
                    if (immersive) {
                        flags = flags or
                            View.SYSTEM_UI_FLAG_LAYOUT_FULLSCREEN or
                            View.SYSTEM_UI_FLAG_LAYOUT_HIDE_NAVIGATION or
                            View.SYSTEM_UI_FLAG_FULLSCREEN or
                            View.SYSTEM_UI_FLAG_HIDE_NAVIGATION or
                            View.SYSTEM_UI_FLAG_IMMERSIVE_STICKY
                    }
                    decor.systemUiVisibility = flags
                }
                invoke.resolve(JSObject().apply { put("immersive", immersive) })
            }
        } catch (error: Exception) {
            invoke.reject(error.message ?: "Failed to change system bars")
        }
    }

    private fun allFilesGranted(): Boolean {
        return if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            Environment.isExternalStorageManager()
        } else {
            true
        }
    }
}

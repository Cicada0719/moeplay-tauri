# 萌游 MoeGame

把游戏、漫画与小说，收进自己的娱乐空间。

萌游是基于 **Tauri 2 + Svelte 5 + Rust** 的本地优先游戏库与 ACG 媒体中心。Windows 管理游戏与资料，Android 提供适合掌机的横屏界面、手柄导航与触控操作。

- 游戏库：本地游戏和模拟器 ROM，封面、收藏、标签与游玩记录。
- 阅读：漫画章节与单页续读，小说按作品整理历史，各章进度独立保存。
- 媒体：番剧搜索与播放，多来源入口、可取消换源和结构化错误恢复；来源可用性取决于对应服务。
- 追番：收藏来源绑定、实际剧集更新检测、未看数和续播目标管理。
- 离线：漫画图片与小说正文按章下载、断点恢复、离线阅读和占用管理。
- 同步与备份：WebDAV 同步保护、番剧/漫画/小说历史 JSON 备份与导入预览。
- 外观：五套主题、壁纸与减少动态效果选项。新安装默认“无界流光”。

## 界面

![无界流光游戏首页（演示资料）](update-server/site/assets/desktop.png)

![小说阅读界面（演示正文）](update-server/site/assets/reading.png)

## 下载

[官网下载页](https://moeplay.sgy0719.top) · [GitHub Releases](https://github.com/Cicada0719/moeplay-tauri/releases) · [更新与问题反馈](https://github.com/Cicada0719/moeplay-tauri/issues)

| 平台 | 选择 |
| --- | --- |
| Windows 10/11 x64 | EXE 安装包、MSI 或 Portable ZIP |
| Android 7.0+ ARM64 | 正式 Release APK，或旧签名兼容 APK |

下载页与 Release 的 `release-manifest.json` 列出实际附件、版本、大小和 SHA-256。以已发布清单为准；仓库版本号不代表已完成发布。

当前开发版本为 v0.24.0，包含可取消播放会话、换源失败终态、源健康状态、追番更新中心、漫画/小说离线阅读、三类历史备份和 WebDAV 写入保护。正式发布前以发布清单和回归报告为准；完整变更见 [CHANGELOG](CHANGELOG.md)。

## 安装与升级

Windows 推荐 EXE；官方安装版通过 HTTPS 检查签名更新，GitHub 是备用入口。Portable 解压后运行，使用前可校验 SHA-256。

Android 正式包与旧版 Debug 包使用不同证书，无法互相覆盖安装。旧用户选择与旧证书一致的兼容包；只有完成对应设备覆盖测试的发布才标注“保留数据升级”。切换签名渠道前，在“统一历史”导出阅读 JSON，再在新安装中导入。这个备份只包含本地漫画/小说阅读历史，不包含完整游戏库、番剧记录、源凭据或设置。

阅读历史首次使用会从旧 localStorage 迁移到 IndexedDB，旧数据保留。漫画/小说 IndexedDB 仍不通过 WebDAV 跨设备接力；WebDAV 只同步已有番剧/SQLite 历史范围。建议在更换 Android 签名渠道前导出三类历史备份。网页与外部漫画阅读器仅恢复章节入口。

## 开发

准备 Node.js 22+、npm、Rust stable。Windows 还需要 Visual Studio C++ Build Tools 与 WebView2；Android 需要 JDK 17、Android SDK 36、NDK 与 Rust `aarch64-linux-android` 目标。

```sh
npm ci
npm run doctor
npm run dev                 # 浏览器 UI，原生能力需 Tauri
npm run tauri -- dev        # 桌面开发
npm run check
npm run test:unit
npm run verify:commands
cargo test --manifest-path src-tauri/Cargo.toml --all-targets
```

浏览器回归：先执行 `npx playwright install chromium`，再执行 `npm run test:visual`。测试使用锁文件对应的 Chromium、本地中文字体和确定性模拟数据；截图必须与已审阅的基准比对。更新基准使用 `--update-snapshots`，提交前检查差异。

## Fork

1. Fork 仓库，克隆你自己的地址，执行 `npm ci`。
2. `npm run tauri -- build` 默认构建不带官方签名更新的安装包，**不需要发布密钥**。
3. 使用 npm 与 `package-lock.json`；不要混用包管理器修改依赖。
4. 分发你自己的产品时修改应用标识、名称、更新公钥和域名，生成自己的签名密钥。

配置参考 [release.config.example.json](release.config.example.json)。私钥、密码、服务器配置保存在仓库外；不要提交 keystore 或带凭据的 URL。源码布局与参与方式见 [CONTRIBUTING](CONTRIBUTING.md)。

## 发布

官方构建通过 `src-tauri/tauri.official.conf.json` 启用签名与更新端点。Windows 使用 `npm run release:win -- --skip-publish`；签名配置沿用仓库外的 `~/.tauri/moeplay-publish-config.json`。

Windows PowerShell 中可执行 `./scripts/build-android.ps1 -Channel Unsigned` 构建未签名 ARM64 APK。需要设置 `JAVA_HOME`、`ANDROID_HOME`、`NDK_HOME`、`LIBCLANG_PATH`；NDK 链接器在 Windows 上要求构建缓存路径为 ASCII，可通过 `-TargetDirectory` 指定。正式签名改用 `-Channel Release`，并从仓库外注入 `ANDROID_KEYSTORE_PATH`、`ANDROID_KEY_ALIAS`、`ANDROID_STORE_PASSWORD`、`ANDROID_KEY_PASSWORD`；`-Channel Compat` 使用本机原有 Debug 证书，分发前必须与旧 APK 核对。默认输出在 `artifacts/<版本>/`，重建时使用新的 `-OutputDirectory`，避免覆盖已验收文件。

发布顺序：本地检查与构建 → 安装验收 → Issue 分支 PR 合并 → 对同一提交创建新 tag → GitHub 上传 → SFTP 上传同批文件 → 验证公网、局域网哈希与更新签名。不要覆盖现有 tag 或附件。

GitHub Release 工作流只校验已上传产物，不另行构建第二批安装包。下载站部署文件位于 `update-server/site/`，Nginx 配置位于 `update-server/deploy/`；服务仅开放网页和下载，上传使用 SSH。

## 许可证

代码：[MIT](LICENSE)。保留第三方依赖与素材的许可证；项目内主题资源附带其来源信息。

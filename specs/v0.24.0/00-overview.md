# v0.24.0 实施总览

## 目标

在 v0.23.1 的播放、阅读历史和下载基础上，交付追番更新中心、漫画/小说章节离线阅读、三类历史备份，并收口播放换源、规则健康、同步写入和发布校验问题。版本号为 `0.24.0`；现有 `docs/` 与旧 `specs/task-*.md` 作为历史契约保留不改。

## 已有能力与边界

- v0.23.1 已有无视频帧监控、旧异步响应丢弃、漫画精确续读、小说按书历史、IndexedDB 阅读历史 v2、普通/番剧持久化下载和视觉主题；不得把这些能力重复实现。
- IndexedDB 阅读位置与番剧 `anime-history` localStorage 仍是本地链路；本版备份直接覆盖两者，不把未完成的 SQLite/WebDAV 同步当作已可用跨端接力。
- 普通下载器已有暂停、续传、重试、重启 hydrate；离线章节必须复用它，不能另造第二套下载队列。
- 本版不做 TXT/EPUB/PDF 导入、完整跨设备自动同步、账号系统或全局 UI 重做。

## 串行任务

`01 playback-runtime` → `02 playback-failover` → `03 source-health` → `04 anime-following` → `05 offline-storage` → `06 offline-reading` → `07 history-backup` → `08 sync-safety` → `09 release-verification` → `10 acceptance-release`。

每项使用独立 GitHub Issue、`feat/issue-<实际编号>` 分支和 PR。前项合并、模块边界核对和验收证据齐全后，才启动下一项。父 Issue：[#14](https://github.com/sgyxyx-prog/moeplay-tauri/issues/14)。

## 共同契约

- 作品身份为 `contentType + sourceId + contentId`，章节身份再加 `chapterId`；标题只展示，不参与唯一性。
- 播放会话必须有 `sessionId`、尝试编号、来源、作品、剧集和恢复位置；所有异步返回携带会话身份。
- 只有真实视频帧解码成功才记录播放源成功；URL、metadata、代理返回不能算成功。
- 离线任务的状态、清单和空间统计与阅读位置分离；下载不会改写阅读历史。
- 失败要区分应用代码、网络/HTTP、源规则和上游站点；不把“未执行”或“源不可达”写成全局源失效。

## 通用门槛

每项新增行为测试，并在提交前运行：`npm run verify:commands`、`cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`、`cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings`、`npm run check`、相关 `cargo test` 与 `npm run test:unit`。真实设备、第三方源和公网发布必须在报告中区分已验证、受限和未执行。


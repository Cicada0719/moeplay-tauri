# 萌游 MoeGame (moeplay-tauri) · 开发进度与问题审计

> 审计日期：2026-07-07 · 审计人：Senior Developer
> 审计范围：当前 working tree（v0.11.5 + 未提交改动）+ CI 质量闸门

## 一、总体进度

- **版本**：v0.11.5，基于 **Tauri 2 + Svelte 5 (Runes) + Rust**，Windows 桌面端。
- **功能矩阵（已实现）**：
  - 游戏管理：Steam / Epic / 模拟器导入、多源刮削（Bangumi·VNDB·DLSite·Steam·PCGW 等）、智能合集、标签/评分/笔记、统计、成就。
  - 番剧播放：多规则源搜索 + 并行换源、WebView 嗅探 + 本地 HTTP 代理、HLS.js/原生双模、弹幕、画中画、倍速、手势、trace.moe 识番。
  - 漫画阅读：在线源聚合、收藏与进度同步。
  - 大屏模式（Big Picture）：PS5 风格导轨、手柄支持、虚拟键盘。
  - 设置 / 备份 / 诊断 / 自动更新。
- **近期重构（0.11.0–0.11.5）**：UI 基元化（Card/Button/Dialog/Input/Switch…）、Store 拆分（gameLibrary/gameSelection）、主题系统（6 模式）、hash 路由持久化、后端安全（路径作用域校验、默认 TLS、ZIP Slip 修复）、跨验证清理（移除 C# 迁移、修复播放器根因、游戏库 QA）。

## 二、本地质量闸门实测结果

| 闸门 | 命令 | 结果 |
|------|------|------|
| Rust 编译 | `cargo check` | ✅ 0 错误 |
| 前端类型 | `npm run check`（svelte-check） | ✅ 0 错误 / 0 警告 |
| 单元测试 | `npm run test:unit`（vitest） | ✅ 107/107 通过（15 文件） |
| **Rust 格式** | `cargo fmt -- --check` | ❌ **167 个 .rs 文件未格式化** |
| **Rust lint** | `cargo clippy -- -D warnings` | ❌ **10 条 lint（9 个 error）** |

> 结论：本地“三闸门”（check / svelte-check / vitest）全绿，但仓库**整体不满足 CI 定义的完整质量闸门**（fmt + clippy 双红）。

## 三、存在的问题（按严重度）

### P0 — 推送 master 必导致 CI 失败
1. **rustfmt 不达标（仓库级）**：`cargo fmt --check` 报告 **167 个文件**需重新格式化，且其中包含**已提交**文件（`anime.rs`、`comic.rs`、`commands/*.rs`、`import.rs`、`integration.rs`、`security.rs`、`scraper/*` 等）。说明整个 Rust 代码库从未被 rustfmt 统一过。CI 的 `cargo fmt -- --check` 步骤会直接失败。
   - 修复：`cargo fmt`（机械操作，一次性大 diff，建议单独提交）。
2. **clippy 不达标**：`cargo clippy -- -D warnings` 报 **10 条 lint**，全部集中在 `src-tauri/src/anime_download.rs`：
   - 5× `manual_strip`（m3u8 解析里手写 `&line["#EXTINF:".len()..]`，应改用 `strip_prefix`）
   - 1× `ptr_arg`（`&PathBuf` 改为 `&Path`）
   - 1× `sort_by_key`、1× `borrowed expression`、1× `very complex type`、1× crate 级 warning
   - 修复：小范围改写，集中在单个文件。

### P1 — 工作区改动易失 / 未固化
3. **53 处未提交改动**（17 修改 / 25 删除 / 2 未跟踪），来自最近的“跨验证清理”工作（移除 C# 迁移、播放器根因修复、游戏库 QA、清理 `.playwright-mcp/` 与截图）。
   - 风险：未提交即丢失；且 CI 永远不会验证这些改动。
   - 建议：确认无误后提交（跨验证报告已附 `plans/cross-validation-report.md`）。

### P2 — 代码卫生
4. **生产代码残留 28 处 `console.log`**（不含测试），集中在播放链路：`anime.svelte.ts`、`AnimePlayer.svelte`、`SourceSheet.svelte`。与项目自身“结构化 JSON 日志”原则（CHANGELOG 0.11.0）及全栈规范冲突，发布后会产生噪声。
   - 建议：改为 `tracing`/调试开关，或移除。
5. **云端同步未完成**：`src-tauri/src/sync.rs:817` 标记 `TODO: 阿里云 OSS 集成`，云存档同步功能尚未实现。

### P3 — 待真机验证的遗留项（来自跨验证报告）
6. **CachedImage 404 兜底**：`onfail` 是否在 HTTP 404 也触发未确认（部分 404 不触发 `onerror`），需在真机/手动核对。
7. **视觉/E2E 冒烟**：`playwright.config.ts` 依赖本机 Chrome（`channel: "chrome"`）与 `?skip_wizard`，校验 `data-testid="app-shell"` / `"main-content"`（已存在）。headless 下 Tauri 端到端无法跑，当前以“静态门禁 + 代码走查 + 手动矩阵”交付。

## 四、建议行动（优先级排序）

1. **先过 CI 格式/lint 闸门**：`cargo fmt` → 修 `anime_download.rs` 的 10 条 clippy lint → 本地重跑 `cargo fmt --check` 与 `cargo clippy -- -D warnings` 全绿。这是推送到 master 前的硬前提。
2. **提交当前 53 处工作树改动**，固化跨验证成果。
3. **清理 28 处 `console.log`**（可合并进同一次格式/清理 PR）。
4. **明确 OSS 同步状态**：实现或显式标记为实验性/未实现，去掉误导性的 TODO 或补充 issue。
5. **按需补真机回归**：播放器起播、CachedImage 404 兜底。

## 五、一句话结论

功能开发进展充分、本地三闸门全绿；但**仓库当前并不满足 CI 的完整质量闸门（rustfmt/clippy 双红），且大量改动未提交**。若直接推送 master，CI 会在 `cargo fmt --check` 阶段失败。建议优先完成“格式化 + clippy 修复 + 提交”三件套，再继续功能迭代。

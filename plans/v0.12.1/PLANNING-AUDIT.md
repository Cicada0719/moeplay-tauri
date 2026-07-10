# 0.12.1 规划交付审计

| 用户要求 | 直接证据 | 判定 |
|---|---|---|
| 以 0.12.1 为开发基线 | `MASTER-PLAN.md` 第 0 节识别当前仍为 0.12.0，并定义 0.12.1 RC 固化步骤 | 已覆盖；不存在虚构 tag |
| 优化四个主功能 | `MASTER-PLAN.md` 第 4 节；`spec-01`、`spec-02`、`spec-04` | 已覆盖：游戏库、记录、番剧、漫画 |
| 增加更多源 | `spec-01-media-sources.md` 第 5–8、13–17 节 | 已覆盖；区分 active/planned/reference、合法性与 kill switch |
| 尽量内部看漫画和番剧 | `spec-01` 第 9–11、14 节 | 已覆盖：resolved target、内部优先、可操作回退、禁止 DRM/付费绕过 |
| 游戏库和仪表盘 UI/功能继续优化 | `spec-02-library-dashboard.md`；`spec-04-ui-design-system.md` | 已覆盖：导入/去重/来源/启动/活动模型/UI/性能 |
| AI 功能完善 | `spec-03-ai-assistant.md` | 已覆盖：Gateway、Secret、schema、任务、三个首批场景、降级 |
| 软件整体 UI 设计完善 | `spec-04-ui-design-system.md` | 已覆盖：审计、token、导航、状态、响应式、大屏、动效、A11y、截图矩阵 |
| 其他小功能 | `MASTER-PLAN.md` Phase 4；`AGENT-WORK-PACKAGES.md` WP-A8；`spec-05` | 已覆盖：下载、备份、诊断、设置、导入、更新/发布 |
| 详细开发 plan | `MASTER-PLAN.md` | 已覆盖：阶段、依赖、预算、门禁、风险、完成定义 |
| 拆分 spec 给多代理 | `AGENT-WORK-PACKAGES.md` + 5 个专项 spec | 已覆盖：A0–A9、独占写入范围、合并顺序、回报模板 |
| 基于当前代码证据 | 各 spec 的“现状证据”；本文件的本地质量记录 | 已覆盖 |

## 当前验证证据

- `npm run check`：通过，0 errors / 0 warnings。
- `npm run test:unit`：155 passed / 1 skipped。
- `npm run test:visual`：3 passed。
- `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`：通过。
- `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings`：通过。
- `cargo test --manifest-path src-tauri/Cargo.toml`：126 passed / 2 ignored。
- `git diff --check -- plans/v0.12.1`：通过。
- 源代码工作树未保留由生成器造成的 EOL 修改；交付范围仅为 `plans/v0.12.1/*.md`。

## 规划中发现的发布前 P0

1. Tauri 注册命令、`build.rs`、capability、前端 command map 不一致；3 个前端已调用命令缺 capability。
2. DB migration 失败路径可能删除主库并静默切换内存库。
3. 多类 Token/API Key 仍在 localStorage/明文 settings JSON。
4. `StatsPage` 与 Rust `DashboardData` DTO 漂移，现有统计 UI 可能消费错误字段。
5. AI 多 Provider 只是部分兼容：本地 Ollama 被 Key 校验阻断，Claude 请求体不匹配，修改 endpoint 可能把旧 Key 发往新 origin。
6. tag release 未复用完整 CI gate，且缺真实 updater 升级验收。

这些 P0 已进入 Master Phase 0/1 与 `spec-05-platform-quality.md`，不允许在正式 0.12.1 发布时跳过。


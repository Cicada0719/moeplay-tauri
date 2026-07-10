# MoePlay 0.12.1 多代理执行与合并规范

## 1. 协作规则

所有代理共享目标，但必须使用**不重叠写入范围**。代理不得回退、格式化或重写不属于自己的文件。若必须修改共享文件，先提交“接口请求”给 Integrator，由 Integrator 统一修改。

### 1.1 分支与提交

- 每个工作包使用 `codex/v0.12.1-<work-package>`。
- 一次提交只覆盖一个工作包；机械格式化与功能提交分开。
- 每个 PR 描述必须包含：目标、数据迁移、feature flag、测试证据、截图、回滚方式。
- 禁止把真实 API Key、Cookie、Token、签名私钥、第三方账号数据加入 fixture。

### 1.2 共享文件所有权

以下文件只允许 Integrator 修改：

- `package.json`、`package-lock.json`
- `src-tauri/Cargo.toml`、`src-tauri/Cargo.lock`
- `src-tauri/tauri.conf.json`
- `src-tauri/src/lib.rs`、`src-tauri/build.rs`
- `src-tauri/capabilities/**`、`src-tauri/permissions/**`
- `src/lib/api/index.ts` 的总导出
- `src/lib/nav.ts`、`src/App.svelte`
- `CHANGELOG.md`、`.github/workflows/**`

各代理如需注册 command、permission、route 或 dependency，应在 spec/PR 中给出精确 patch 请求，由 Integrator 在合并阶段一次完成。

## 2. 代理拓扑

| Agent | 工作包 | 可并行阶段 | 依赖 |
|---|---|---|---|
| A0 | Integrator / Release Captain | 全程 | 无 |
| A1 | Domain Contracts & Migrations | Phase 1 | A0 |
| A2 | Game Library | Phase 2 | A1 |
| A3 | Activity Dashboard | Phase 2 | A1 |
| A4 | Anime Provider & Playback | Phase 2 | A1 |
| A5 | Comic Provider & Reader | Phase 2 | A1 |
| A6 | AI Gateway & Experiences | Phase 3（部分可提前） | A1、A2/A3 数据接口 |
| A7 | UI Design System & Accessibility | Phase 2–4 | A1 页面状态接口 |
| A8 | Jobs / Downloads / Backup / Diagnostics | Phase 2–4 | A1 |
| A9 | QA / Performance / Security / Release | 全程旁路 | A0，最终依赖所有 |

## 3. 工作包定义

## WP-A0 — Integrator / Release Captain

**目标**：守住主干、共享接口、版本和发布一致性。

**写入范围**：共享文件清单、`plans/v0.12.1/**`、最终冲突文件。

**任务**：

1. 建立 0.12.1 rc 分支和 feature flags。
2. 统一版本号、CHANGELOG、配置与更新器发布路径。
3. 定义 Rust/TypeScript 共享错误码、DTO 命名和 command 注册规则。
4. 审核所有 migration、capability、依赖和 source license。
5. 按顺序合并：A1 → A2/A3/A4/A5 → A6/A7/A8 → A9。
6. 每轮合并后运行完整门禁。

**验收**：主干始终可构建；不存在半注册 command 或版本不一致。

## WP-A1 — Domain Contracts & Migrations

**目标**：为四主功能提供统一 Provider、Progress、Activity、Health、Job、Secret 接口。

**独占写入范围**：

- `src-tauri/src/domain/**`（新）
- `src-tauri/src/repositories/**`（新）
- `src-tauri/src/migrations/**`（新）
- `src/lib/domain/**`（新）
- `src/lib/api/contracts/**`（新）
- 对应 tests/fixtures

**不得直接改**：现有 anime/comic/game 页面和共享注册文件。

**任务**：

- 定义领域 DTO 和稳定序列化格式。
- 设计显式 SQL migration ledger；兼容现有 `SCHEMA_VERSION=2` 与 JSON `CURRENT_SCHEMA_VERSION=1`。
- 创建 ActivityEvent、ProgressRecord、ProviderHealth、BackgroundJob、SecretRef schema。
- 建立 normalized error 与 cancellation contract。
- SecretStore 使用系统 keyring；测试环境使用内存实现。

**交付物**：migration plan、golden DB、contract tests、共享接口文档。

**验收**：旧库副本升级成功；失败事务不改变原库；secret redaction 通过。

## WP-A2 — Game Library

**目标**：完成导入—整理—刮削—启动—记录闭环。

**独占写入范围**：

- `src/lib/features/library/**`（新）
- `src-tauri/src/services/library/**`（新）
- `src-tauri/src/providers/game/**`（新）
- `tests/visual/library-*.spec.ts`

**迁移策略**：旧 `SwitchHome/GameGrid/GameDetailPage/gameLibrary store` 先作为 adapter；最终迁移由 A0/A7 完成。

**任务组**：

- A2.1 Import diff preview：新增/更新/冲突/忽略。
- A2.2 Identity & dedupe：路径、平台 ID、title fingerprint。
- A2.3 Field provenance：刮削字段来源/置信度/回滚。
- A2.4 Batch actions：标签、状态、合集、封面、失效路径。
- A2.5 Launch diagnostics：客户端、路径、locale、权限。
- A2.6 Virtualized library：1000/5000 项性能。

**验收**：重复导入幂等；批量操作可撤销；启动失败可诊断；1000 项预算达标。

## WP-A3 — Activity Dashboard

**目标**：把记录页变成统一活动与继续中心。

**独占写入范围**：

- `src/lib/features/activity/**`（新）
- `src-tauri/src/services/activity/**`（新）
- `tests/visual/activity-*.spec.ts`

**任务组**：

- A3.1 游戏 session → ActivityEvent。
- A3.2 番剧/漫画 progress → ActivityEvent。
- A3.3 聚合查询与索引。
- A3.4 时间/类型筛选、继续入口、趋势和数据解释。
- A3.5 记录编辑、合并、删除、导出、隐私模式。
- A3.6 空数据/旧数据/时区/DST 测试。

**验收**：20000 事件首屏 ≤ 800ms；继续目标准确；编辑后聚合一致。

## WP-A4 — Anime Provider & Playback

**目标**：番剧多源可搜索、可选集、可稳定起播并自动恢复。

**独占写入范围**：

- `src/lib/features/anime/**`（新）
- `src-tauri/src/providers/anime/**`（新）
- `src-tauri/tests/fixtures/anime/**`
- `tests/visual/anime-*.spec.ts`

**任务组**：

- A4.1 拆分 Kazumi rule connector、metadata provider、playback resolver。
- A4.2 增量规则同步、schema validation、签名/来源展示。
- A4.3 Jellyfin connector（用户自有库）与本地视频 provider。
- A4.4 Episode identity：season/episode/absolute number/title alias。
- A4.5 Resolver descriptor：HLS/file/webview/external。
- A4.6 Health、熔断、验证、换源和错误面板。
- A4.7 播放进度、弹幕偏移、画中画、外部播放器一致性。

**验收**：单源失败不阻断聚合；换源不串集；3 个 fixture 场景和 live acceptance 通过。

## WP-A5 — Comic Provider & Reader

**目标**：漫画多源可搜索、可阅读、可恢复进度，并支持自托管库。

**独占写入范围**：

- `src/lib/features/comic/**`（新）
- `src-tauri/src/providers/comic/**`（新）
- `src-tauri/tests/fixtures/comic/**`
- `tests/visual/comic-*.spec.ts`

**任务组**：

- A5.1 统一 MangaDex/Baozi/DM5/1kkk adapter。
- A5.2 Komga/Kavita connectors；LANraragi/Suwayomi 作为后续 feature flag。
- A5.3 OPDS/local CBZ/PDF connector。
- A5.4 Chapter identity、排序、分页和语言/分组。
- A5.5 Reader prefetch、缓存限额、单图重试、长条/分页。
- A5.6 Progress、书签、阅读方向和章节自动切换。
- A5.7 成人源隔离与配置/历史隐私。

**验收**：首图预算达标；单页失败不丢进度；缓存清理不删收藏/历史；自托管 connector contract 通过。

## WP-A6 — AI Gateway & Experiences

**目标**：把 AI 变成可关闭、可解释、可确认的增益层。

**独占写入范围**：

- `src-tauri/src/ai/**`（新）
- `src/lib/features/ai/**`（新）
- `src-tauri/tests/fixtures/ai/**`
- `tests/visual/ai-*.spec.ts`

**任务组**：

- A6.1 Gateway：OpenAI-compatible、Ollama/local、provider capability。
- A6.2 Prompt registry + JSON schema + model policy。
- A6.3 SecretRef、budget、rate limit、cancel、redaction。
- A6.4 Library cleanup change set。
- A6.5 Natural language filter compiler。
- A6.6 Recommendation explanation engine（规则/历史先于 LLM）。
- A6.7 摘要/翻译/标签/笔记 patch 与 undo。

**验收**：AI 离线时主功能无回退；非法 JSON 不写库；用户确认前零写入；key 不返回前端。

## WP-A7 — UI Design System & Accessibility

**目标**：统一四主功能 UI，而不是做一次换皮。

**独占写入范围**：

- `src/lib/components/ui-v2/**`（新）
- `src/lib/styles/**`（新）
- `src/lib/actions/a11y/**`（新）
- `tests/visual/baselines/**`

**任务组**：

- A7.1 tokens：color/type/space/radius/elevation/motion/density。
- A7.2 PageShell/Header/FilterBar/AsyncState/DetailPanel。
- A7.3 键盘、焦点、Dialog/Drawer、screen reader 文案。
- A7.4 reduced-motion 和 GSAP cleanup utilities。
- A7.5 responsive：900/1200/1600/4K、125%/150% scale。
- A7.6 Big Picture 10-foot UI 和 gamepad focus map。
- A7.7 逐页迁移清单和截图基线。

**验收**：四主页面状态一致；键盘完成主流程；截图矩阵无意外差异；无持续动画 CPU 泄漏。

## WP-A8 — Jobs / Downloads / Backup / Diagnostics

**目标**：把“小功能”接入同一任务、错误和诊断体系。

**独占写入范围**：

- `src-tauri/src/jobs/**`（新）
- `src/lib/features/jobs/**`（新）
- `src/lib/features/diagnostics/**`（新）
- 相关 tests

**任务组**：

- A8.1 可持久任务队列、取消/暂停/重试、并发限制。
- A8.2 番剧/漫画下载与封面/刮削任务统一。
- A8.3 Backup/restore 加入 schema/version/hash/preview。
- A8.4 Diagnostics bundle 默认脱敏。
- A8.5 下载磁盘配额、空间预检、失败恢复。
- A8.6 未完成 OSS 同步显式标记实验性，避免误导。

**验收**：重启恢复任务；取消不留下错误状态；诊断包不含 secret；恢复前可预览。

## WP-A9 — QA / Performance / Security / Release

**目标**：独立验证，不以开发代理自测替代验收。

**独占写入范围**：

- `tests/contracts/**`
- `tests/performance/**`
- `tests/security/**`
- `scripts/verify-*.ps1`
- 报告 `plans/v0.12.1/verification-report.md`

**任务组**：

- A9.1 维护 requirement → evidence traceability matrix。
- A9.2 Provider fixture/live acceptance 分层。
- A9.3 1000/5000 游戏、20000 活动、长章节 benchmark。
- A9.4 URL/SSRF/path traversal/zip slip/secret redaction tests。
- A9.5 安装升级、便携、自动更新、回滚。
- A9.6 检查 MSI/NSIS/updater signature/manifest/hash。

**验收**：每个显式 requirement 都有直接证据；未验证项不得标为完成。

## 4. 并行排期建议

```text
Day 1-2   A0 baseline/release + A1 contracts/migrations + A9 baseline evidence
Day 3-5   A1 完成；A7 建 tokens/shell；A2/A3/A4/A5 写 adapters
Day 6-10  A2/A3/A4/A5 主功能并行；A8 jobs；A9 contract/perf harness
Day 11-14 A6 AI；A7 逐页迁移；A4/A5 live acceptance
Day 15-17 集成、修复、数据迁移演练、全量截图/性能
Day 18-20 RC、签名构建、升级/回滚、release notes
```

排期按 6–10 个有效开发代理估算。若只有 3–4 个代理，优先级为：A1 → A4/A5 → A2/A3 → A6 → A7/A8 → A9，不得跳过 A9。

## 5. 合并顺序与冲突处理

1. A1 只提供新 contract，不立即删除旧 API。
2. A2–A5 以 adapter 接入新 contract，feature flag 默认关闭。
3. A7 提供 UI v2 primitives，各功能代理只消费，不私自复制样式。
4. A6/A8 复用 A1 jobs/secrets/errors，不另建第二套。
5. A0 在单独 integration commit 注册 command/capability/export/navigation。
6. A9 对 integration commit 做独立审计；发现缺证据则退回对应工作包。
7. 全部门禁通过后才删除旧路径和 feature flag。

## 6. 代理完成回报模板

```markdown
### Work package
WP-Ax / task IDs

### Changed files
- absolute/relative path

### Behavior
- before / after

### Data & migration
- schema/version/rollback

### Tests
- command + result

### Shared-file requests
- exact command/export/dependency/capability changes needed from A0

### Risks / remaining
- explicit, no hidden TODO
```

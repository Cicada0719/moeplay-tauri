# 子任务 7 开发规格说明：执行全量回归与发布

## 1. 任务目标

在任务 2/3/5/6 全部完成后，建立并执行覆盖三端（Windows/macOS/Linux）的全量回归测试体系，包括：单元测试聚合、源切换抽样验证、v1→v2 数据迁移演练（含回滚路径）、双设备 WebDAV 同步端到端验证、视觉冒烟测试与冷启动性能测量。全部通过后完成版本号升级（v2.0）、CHANGELOG 编写、打 tag 触发 `release.yml` 自动构建发布，并完成灰度观察。本子任务的核心交付是**一套可重复执行的回归套件 + 一份发布 Runbook**，而非新功能代码。

## 2. 上下文与约束

- **关联 PRD 功能**：本任务验证 §1.3 全部成功指标，直接覆盖 FR-01~FR-11 的验收标准汇总；里程碑 M7（§7）。
- **技术栈**：Tauri 2 + Svelte 前端（Node 侧测试用 vitest），Rust 侧测试用 `cargo test`；CI 为 GitHub Actions（现有 `.github/workflows/release.yml` 主流程**原则上不改动**，仅允许按 §8 禁止修改清单中定义的例外微调）。
- **现有代码结构假设**（若实际不符，以实际为准并记录在回归报告中）：
  - 前端测试：`tests/`（vitest 单元测试）、`tests/visual/`（新增，视觉冒烟）
  - Rust 测试：`src-tauri/tests/`（集成测试）
  - 规则目录：`resources/rules/`（任务 2 产出）
  - 数据库：SQLite（任务 4 产出）
- **约束**：
  - 本任务**不修复缺陷**：回归中发现的 bug 一律记录到 `REGRESSION_REPORT.md` 并回退给对应任务负责人修复，本任务仅允许新增测试、脚本、文档与 CI 配置。
  - 性能门槛：冷启动 ≤ 3s（中等配置 Windows 实测）；规则包加载 ≤ 500ms。
  - 三端产物必须与现有 release.yml 产物形态一致（msi/dmg/AppImage/deb）。
  - 灰度观察期 1 周，期间 tag 不可删除，发现问题走 hotfix tag（v2.0.1）流程。

## 3. 输入 / 输出

### 3.1 输入（依赖前置任务产出的接口，详见 §5）

| 输入 | 来源 | 说明 |
|---|---|---|
| 规则健康检查 CLI | 任务 2 | Tauri 命令 `check_source_health(sourceId)` 或独立 CLI |
| 规则包版本信息 | 任务 2 | Tauri 命令 `get_rule_bundle_info() -> { version, updatedAt }` |
| 数据迁移 API | 任务 4 | `run_migration()` / `rollback_migration()` / 备份目录 `backup/` |
| WebDAV 同步 API | 任务 5 | `sync_now() -> SyncResult { uploaded, downloaded, conflicts }` |
| 播放器/双页阅读 UI | 任务 3/6 | 可手动操作的发布候选构建 |

### 3.2 输出（本任务新增的文件/制品）

| 路径 | 类型 | 说明 |
|---|---|---|
| `tests/regression/source-switch.sample.ts` | 脚本 | 源切换抽样测试（§4 步骤 3） |
| `tests/regression/migration-drill.ts` | 脚本 | 迁移演练驱动脚本 |
| `tests/regression/fixtures/v1-history/*.json` | 数据 | v1 历史数据样本（正常/损坏/超大 3 套） |
| `tests/regression/webdav-e2e/` | 脚本+编排 | 双设备同步 E2E（docker-compose + 驱动脚本） |
| `tests/visual/smoke.spec.ts` | 测试 | 视觉冒烟（关键页面截图比对） |
| `tests/regression/cold-start.ts` | 脚本 | 冷启动耗时测量（三端） |
| `scripts/release-checklist.ts` | 脚本 | 发布前自动检查聚合入口 |
| `.github/workflows/regression.yml` | CI | 回归工作流（**新增文件**，不动 release.yml 主逻辑） |
| `REGRESSION_REPORT.md` | 文档 | 回归结果与缺陷清单 |
| `RELEASE_RUNBOOK.md` | 文档 | 发布操作手册（打 tag、回滚、hotfix） |
| `CHANGELOG.md` | 文档 | v2.0 更新日志 |

### 3.3 关键数据结构

```ts
// tests/regression/types.ts
export interface RegressionResult {
  caseId: string;            // 如 "SS-ANIME-03"
  category: 'source-switch' | 'migration' | 'sync' | 'player' | 'reader' | 'perf';
  platform: 'windows' | 'macos' | 'linux';
  passed: boolean;
  durationMs: number;
  detail?: string;           // 失败时填错误摘要
  screenshotPath?: string;   // 视觉冒烟失败时附截图
}

export interface SourceSwitchSampleResult {
  contentType: 'anime' | 'manga' | 'novel';
  sourceFrom: string;
  sourceTo: string;
  itemTitle: string;
  positionKept: boolean;     // 集/话/章 + 进度是否保持（±10s 容差）
  firstPlaySuccess: boolean;
  crashOrBlank: boolean;
}

export interface SyncE2EResult {
  scenario: string;          // 见 §6 同步用例表
  converged: boolean;        // 两端最终数据一致
  attempts: number;          // 同步执行次数（幂等验证）
  conflicts: number;
  success: boolean;
}
```

## 4. 实现步骤

> 按顺序执行。步骤 1~6 可三端并行，步骤 7 为发布闸门。

### 步骤 1：聚合全部既有测试并接入 CI

1. 确认根 `package.json` 存在脚本：`test:unit`（vitest run）、`test:rust`（`cd src-tauri && cargo test --all-features`）。若无则添加。
2. 新增 `scripts/release-checklist.ts`（Node 脚本，用 `execa` 顺序执行）：
   - `pnpm test:unit`（或 npm/yarn，以仓库实际包管理器为准）
   - `pnpm test:rust`
   - `pnpm tsc --noEmit`（前端类型检查）
   - `cd src-tauri && cargo clippy -- -D warnings`
   - 任一失败即非零退出，输出 JSON 汇总到 `tests/regression/results/unit-summary.json`。
3. 新增 `.github/workflows/regression.yml`：
   - 触发：`workflow_dispatch` + `push` 到 `release/**` 分支 + tag `v*` 前置 job。
   - matrix：`os: [windows-latest, macos-latest, ubuntu-latest]`。
   - 步骤：checkout → 安装 Rust/Node → 缓存 cargo/npm → 运行 `scripts/release-checklist.ts` → 上传 `results/` 为 artifact。

### 步骤 2：迁移演练（FR-08 验收）

1. 制作 fixtures：`tests/regression/fixtures/v1-history/`
   - `normal.json`：番剧/漫画/小说混合 200 条合法 v1 记录。
   - `corrupt.json`：含缺字段、非法 JSON 片段、重复 id 的样本。
   - `large.json`：脚本生成 10000 条（写生成器 `fixtures/gen-large.ts`）。
2. 新增 `tests/regression/migration-drill.ts`，流程：
   ```
   for fixture in [normal, corrupt, large]:
     1. 准备干净应用数据目录（临时目录注入 v1 数据）
     2. 启动应用（或调用迁移模块入口 run_migration()）
     3. 断言：v2 记录数 == 预期；backup/ 目录存在且内容与原始一致
     4. corrupt 用例：断言回滚触发、用户提示文案出现、原始数据未被破坏
     5. large 用例：断言迁移耗时 < 30s，无重复记录（按 id 去重校验）
     6. 中断演练：迁移进行中 kill 进程 → 重启 → 断言断点续迁且无重复（FR-08 第三条验收）
   ```
3. 结果写入 `tests/regression/results/migration.json`。成功率必须 100%，否则阻塞发布。

### 步骤 3：源切换抽样验证（FR-02 + 成功指标）

1. 新增 `tests/regression/source-switch.sample.ts`：
   - 抽样策略：对每类内容（anime/manga/novel）从内置源中各取全部源，每类固定 10 个条目（番剧用 PRD 关键词"进击的巨人"等价条目；漫画/小说各选榜单前 10）。
   - 自动化部分（headless，走 Rust 侧引擎接口而非 UI）：对每个条目执行 `search → detail → chapter → parse → 模拟切换到每个其他源 → 断言新源定位与进度保持（positionKept）`，输出 `SourceSwitchSampleResult[]`。
   - 半自动部分（UI 层崩溃/白屏检测）：用 tauri-driver（WebDriver）驱动真实窗口，执行快速连切 3 次用例（FR-02 竞态），断言窗口仍可交互、DOM 无错误页。若 tauri-driver 集成成本超 1 天，降级为手动执行 + 录屏存档，记录在 Runbook。
2. 断言门槛：每类 `firstPlaySuccess ≥ 90%`、`crashOrBlank == 0`。任一不达标 → 阻塞发布，缺陷记入报告。

### 步骤 4：双设备 WebDAV 同步 E2E（FR-09 验收）

1. 新增 `tests/regression/webdav-e2e/docker-compose.yml`：起一个 WebDAV 服务（镜像 `bytemark/webdav` 或等价），固定端口与测试凭据，数据卷临时化。
2. 新增 `tests/regression/webdav-e2e/run.ts`：
   - 模拟"两台设备" = 两个独立的应用数据目录（`device-a/`、`device-b/`）+ 两个不同 `device_id`，直接调用任务 5 的同步核心模块（非 UI）。
   - 用例表（对应 §6 测试要求）：
     | # | 场景 | 预期 |
     |---|---|---|
     | S1 | A 写入第 8 集(今天10:00)，B 写入第 5 集(昨天20:00)，双向同步 | 两端均第 8 集 |
     | S2 | 凭据错误 | 明确认证错误，本地数据零变更 |
     | S3 | 同步中途断网（脚本停容器）→ 恢复 → 再同步 | 最终一致、无重复（幂等） |
     | S4 | A 删除一条（墓碑），同步后 B 同步 | B 上该条同样删除 |
     | S5 | 同秒冲突 | 保留 progress 更大一方 |
     | S6 | 双向各新增 50 条，同步 | 上传/下载计数正确，结果 `uploaded/downloaded/conflicts` 展示值正确 |
   - 每个用例重复 20 轮统计成功率，要求 ≥ 95%。
3. 结果写入 `tests/regression/results/sync-e2e.json`。

### 步骤 5：播放器与阅读器回归（FR-06/07/10/11）

1. `src-tauri/tests/player_idle_regression.rs` 或前端 `tests/player/idle-timer.spec.ts`（按任务 3 实际实现位置选择其一，不重复造）：
   - 模拟画质 普清↔超清 来回切换 5 次，每次模拟鼠标静止 3s，断言控制栏隐藏（若 UI 层难测，测任务 3 交付的 `useIdleTimer` 状态机：注入假时钟，断言 `hidden` 状态迁移）。
   - 菜单展开时静止超时不隐藏、关闭后重新计时。
2. 手动验证单（写入 `RELEASE_RUNBOOK.md` 附录，需真人执行并勾选）：
   - 超清全屏 3s 隐藏 / 移动 200ms 内复现（FR-06 前两条）。
   - 403 播放地址 → 2s 内错误降级 UI + "切换源"按钮可用（FR-07）。
   - 漫画双页：RTL 翻页方向、跨页大图独占、窗口 <800px 提示、退出重进配对不串页（FR-11 全部 Given/When/Then）。
   - 漫画续读 ±0 页、小说续读 ±5%（FR-10）。
3. 视觉冒烟 `tests/visual/smoke.spec.ts`：对首页、源列表、播放器、漫画阅读器（单页/双页）、历史列表、设置页各截 1 张基准图，与 `tests/visual/baseline/` 比对（像素容差 1%）。首跑生成 baseline，后续回归比对。

### 步骤 6：性能测量

1. `tests/regression/cold-start.ts`：三端各冷启动 5 次取中位数（从进程启动到首屏 `DOMContentLoaded` + 主列表渲染完成打点；前端在首屏组件 `onMount` 里 `performance.now()` 上报到 stdout/日志文件）。
2. 断言：中位数 ≤ 3s；规则包加载（30 条）≤ 500ms（读取任务 1 引擎暴露的加载耗时日志断言）。
3. 历史列表 1 万条滚动：用 large fixture 迁移后的库，手动 + 录屏确认滚动流畅、首屏 ≤ 300ms。

### 步骤 7：版本升级与发布

1. 确认 §7 DoD 全部勾选后：
   - `package.json`、`src-tauri/Cargo.toml`、`src-tauri/tauri.conf.json` 版本统一改为 `2.0.0`。
   - 编写 `CHANGELOG.md`（Keep a Changelog 格式）：按 FR-01~FR-11 分组列出 Added/Fixed/Changed，注明 v1→v2 数据迁移行为与备份位置。
   - 更新 `REGRESSION_REPORT.md` 最终版（所有结果 JSON 汇总成表 + 遗留已知问题清单）。
2. 按 `RELEASE_RUNBOOK.md` 执行：
   ```
   git checkout master && git pull
   git tag -a v2.0.0 -m "moeplay v2.0: 规则引擎重建/播放器修复/历史同步/双页阅读"
   git push origin v2.0.0
   ```
3. 监控 `release.yml` 三端构建全部成功；下载三端产物在干净机器/VM 安装冒烟（安装→启动→搜索→播放→历史→同步配置，每端 15 分钟检查单，附在 Runbook）。
4. 灰度观察 1 周：观察 GitHub Issues/崩溃日志，无 P0/P1 缺陷则发布完成；出现阻塞缺陷走 Runbook 的 hotfix 流程（v2.0.1 tag）。

## 5. 依赖的外部接口（接口假设与对接方式）

| 依赖任务 | 假设接口 | 对接方式 | 不满足时的处理 |
|---|---|---|---|
| 任务 2 | Tauri 命令 `check_source_health(source_id) -> { ok, latencyMs, error? }`；`list_sources(content_type) -> Source[]` | 步骤 3 抽样脚本通过 `tauri::test` 或直接调用引擎 crate 函数 | 若仅有 CLI，改用子进程调用并解析 JSON 输出 |
| 任务 4 | Rust 函数 `migration::run(data_dir) -> MigrationReport { migrated, backupPath, rolledBack }`，支持断点续迁 | 步骤 2 以 `src-tauri/tests/` 集成测试方式调用 | 若只暴露 Tauri 命令，用 `tauri::test::mock_app` 调 invoke |
| 任务 5 | Rust 模块 `sync::{SyncEngine, SyncConfig}`，`SyncEngine::sync() -> SyncResult`；凭据走 keyring，测试支持注入内存凭据提供器 | 步骤 4 直接实例化两个 `SyncEngine`（不同 data_dir/device_id） | 若凭据无法注入，E2E 降级为手动双机执行并录屏 |
| 任务 3 | 前端 `useIdleTimer` action 可注入时钟/事件源（或暴露内部 store） | 步骤 5 单测注入 fake timers | 若不可注入，仅保留手动验证单并在报告中标注覆盖缺口 |
| 任务 6 | 历史列表/阅读器路由可带参数直达（如 `/reader?contentId=..&page=7`） | 视觉冒烟与续读验证用 URL 直达 | 不支持则脚本内模拟点击流 |

所有接口假设在任务开始时用半天时间与各任务产出物核对，偏差记录在 `REGRESSION_REPORT.md` 的"接口核对"章节。

## 6. 测试要求

### 6.1 自动化用例清单

| 类别 | 用例 | 路径 | 门槛 |
|---|---|---|---|
| 正常 | 全量单元测试 + clippy | `scripts/release-checklist.ts` | 100% 通过 |
| 正常 | 迁移 normal/large fixtures，条数一致、备份存在 | `tests/regression/migration-drill.ts` | 100% |
| 异常 | 迁移 corrupt fixture 触发回滚，原始数据无损 | 同上 | 100% |
| 异常 | 迁移中断（kill）→ 重启断点续迁，无重复 | 同上 | 100% |
| 正常 | 源切换抽样：每类 10 条目 × 全部互切 | `source-switch.sample.ts` | 首次播放成功率 ≥90% |
| 异常 | 快速连切 3 次仅最后一次生效，无崩溃/白屏 | 同上（tauri-driver） | 崩溃 0 次 |
| 正常 | 同步 S1/S4/S5/S6 | `webdav-e2e/run.ts` | 成功率 ≥95% |
| 异常 | 同步 S2 错误凭据、S3 断网幂等 | 同上 | 数据零丢失 |
| 正常 | 画质切换 5 次控制栏均隐藏；菜单展开不隐藏 | `tests/player/idle-timer.spec.ts` | 100% |
| 边界 | 跨页大图独占、窗口 <800px 提示（视觉冒烟截图断言 DOM 类名） | `tests/visual/smoke.spec.ts` | 100% |
| 性能 | 冷启动 ≤3s；规则加载 ≤500ms | `cold-start.ts` | 中位数达标 |

### 6.2 手动验证单（必须真人执行并签字记录于 REGRESSION_REPORT.md）

- FR-06 全屏隐藏/复现延迟体感验证（三端各 1 次）。
- FR-07 403 降级 UI + 重试 2 次后源推荐弹窗。
- FR-10 漫画 ±0 页、小说 ±5% 续读精度。
- FR-11 RTL 方向映射、双页持久化。
- 三端产物干净机器安装冒烟（每端 15 分钟检查单）。

## 7. 完成定义（Definition of Done）

- [ ] `scripts/release-checklist.ts` 在三端 CI（regression.yml matrix）全部绿灯，结果 artifact 已归档。
- [ ] 迁移演练 4 类用例（正常/损坏/超大/中断）100% 通过，含回滚路径实测证据。
- [ ] 源切换抽样：每类 10 条目，首次播放成功率 ≥90%，崩溃/白屏 = 0；竞态用例通过。
- [ ] WebDAV E2E 6 场景 × 20 轮成功率 ≥95%，S2 凭据错误零数据变更已验证。
- [ ] 播放器控制栏回归（含超清 5 次切换）与视觉冒烟全部通过，baseline 已入库。
- [ ] 冷启动 ≤3s、规则加载 ≤500ms 实测数据记录于报告。
- [ ] 手动验证单全部勾选，三端安装冒烟通过。
- [ ] 版本号三处统一为 2.0.0；`CHANGELOG.md`、`REGRESSION_REPORT.md`、`RELEASE_RUNBOOK.md` 完成并提交。
- [ ] tag `v2.0.0` 推送后 release.yml 三端产物构建成功。
- [ ] 灰度观察 1 周无 P0/P1 缺陷（或已通过 hotfix 闭环）。

## 8. 禁止修改清单

- ❌ **禁止修改** `.github/workflows/release.yml` 的构建/打包/上传产物逻辑（唯一允许：在 release.yml 开头追加一个调用 regression.yml 的前置 `workflow_call` job，且必须经评审；若评审不通过则完全不动）。
- ❌ 禁止修改 `src-tauri/src/` 与 `src/` 下任何**功能代码**（本任务不修 bug；发现的缺陷一律记报告并回退任务 2/3/5/6 处理）。
- ❌ 禁止修改 `resources/rules/` 下的规则文件内容（源失效问题走任务 2 的热更新流程，不在本任务手改）。
- ❌ 禁止修改任务 4 的迁移逻辑与任务 5 的合并算法（即使演练暴露问题，也只提缺陷单）。
- ❌ 禁止向仓库提交任何真实 WebDAV 凭据、用户数据样本；fixtures 必须为合成数据。
- ❌ 禁止跳过灰度观察直接宣布发布完成；禁止 force-push 或删除已发布的 `v2.0.0` tag。
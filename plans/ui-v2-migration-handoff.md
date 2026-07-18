# MoeGame ui-v2 全站迁移 · 交接执行方案（批次 3 + 收尾）

> 本文档是自包含执行方案，交给任何具备文件读写与命令执行能力的 AI 即可开工。
> 执行者无需了解此前对话，严格按本文档操作即可。

---

## 0. 项目与当前状态

- **仓库**：`D:\我的文件\桌面备份\hermes\moeplay-tauri`（Windows 工作区）
- **栈**：Tauri 2 + Svelte 5（Runes 语法）+ TypeScript + Vite，桌面应用「萌游 MoeGame」
- **当前基线（全部已验证为绿）**：
  - `npm run check`（svelte-check）：0 错误 0 警告
  - `npm run test:unit`：101 个测试文件 / 467 用例全过（1 个 liveAcceptance 网络测试跳过，属原状）
  - `npm run build`：构建成功（chunk >500kB 体积警告为既有现象，非错误）
- **已完成迁移的 7 个页面**（都有对应契约测试，可作为模板参考）：

| 页面 | 契约测试 |
|---|---|
| SettingsPage（+ settings/ 四个子组件） | `src/lib/components/settings/ui-v2-migration.test.ts` |
| DiscoveryPage | `src/lib/components/discovery-ui-v2-migration.test.ts` |
| StatsPage | `src/lib/components/stats-ui-v2-migration.test.ts` |
| DownloadPage | `src/lib/components/downloads-ui-v2-migration.test.ts` |
| BackupPage | `src/lib/components/backup-ui-v2-migration.test.ts` |
| DiagnosticsPage | `src/lib/components/diagnostics-ui-v2-migration.test.ts` |
| NovelPage | `src/lib/components/novel-ui-v2-migration.test.ts` |

- **本方案剩余范围（批次 3）**：
  - `src/lib/components/PlatformImportPage.svelte`（约 1446 行）
  - `src/lib/components/GameDetailPage.svelte`（约 930 行）

---

## 1. 铁律（不可违反）

1. **只改 `src/` 前端**。禁止动 `src-tauri/`（Rust）、禁止改任何版本号（package.json / Cargo.toml / tauri.conf.json）。
2. **业务行为零改动**。只迁移页面骨架、三态、样式接入与文案；导入、刮削、下载、备份、播放等业务逻辑函数一行不改。
3. **编辑前必须先 Read 目标文件**，禁止凭记忆写 old_string。
4. 部分文件（如 `settings.svelte.ts`、`runtime.svelte.ts`）是 CRLF/LF 混合行尾，Edit 失败时用字节级脚本替换，不改变其余部分行尾。
5. **CSP 安全**：不引入外部资源、不写内联脚本。
6. **reduced-motion 双写**：每个动效同时写 `@media (prefers-reduced-motion: reduce)` 和 `:global([data-motion="reduce"])` 降级。
7. **动画只用 transform / opacity**。
8. platform 抽象一律 `from "../platform"`（或 `"../../platform"`）导入；**禁止直接 import `platform/windows/` 或 `platform/android/` 内部文件**。

---

## 2. 单页四步配方（SOP，已在 7 页上验证）

### Step 1 · 死链路核查

逐项检查并随迁移清理，结论写进汇报（没有问题就写"核查无死链"）：

- [ ] 无 `onclick` 的 `<button>` / 带 `hoverable` 但无点击行为的卡片（假交互暗示）
- [ ] 未被任何模板消费的 `$state` / derived / import
- [ ] 选择器在标记中不存在的死 CSS（注意排除 app.css 全局同名规则）
- [ ] **互斥分支吞错**：`{#if report}…{:else if error}` 结构中 report 就绪后 error 分支永不可达，导致失败被静默吞掉（真实案例：DiagnosticsPage 导出失败被吞，需拆独立 error state）
- [ ] 首载假空态：数据返回前先闪"暂无内容"（需加 initialLoading 门闩）
- [ ] 重复的顶部 error banner（与各分区三态重复时折叠进三态）
- [ ] 被 router/测试消费的 `data-*` 属性（如 `data-route-scroll`、`data-search-scope`、`data-testid`）**必须保留，勿当死链删**

### Step 2 · ui-v2 迁移

ui-v2 原语位于 `src/lib/components/ui-v2/`：`PageShell / PageHeader / AsyncSection / AsyncState / StateBoundary / ContentGrid / DetailPanel / Drawer / FilterBar / MediaCard / MediaRow`（index.ts 统一导出）。

**骨架模板**（与已完成页面一致）：

```svelte
<PageShell as="div" width="full" scrollable={false} class="xxx-v2-shell" labelledBy="xxx-page-title" ariaLabel={i18n.t("xxx.title")}>
  <div class="页面根">
    <div class="v2-grain xxx-grain" aria-hidden="true"></div>
    <PageHeader id="xxx-page-title" eyebrow="假名 / ENGLISH" title={i18n.t("xxx.title")} description={i18n.t("xxx.subtitle")} />
    <!-- 内容：三态原语包裹 -->
  </div>
</PageShell>
```

- `.v2-grain` 网点工具类已存在于 `src/lib/styles/tokens-v2.css`，直接加背景层 div 即可。
- **eyebrow 配对参照**（假名 / 英文）：设置 `設置 / SETTINGS`、发现 `発見 / DISCOVERY`、统计 `統計 / STATS`、下载 `ダウンロード / DOWNLOADS`、备份 `バックアップ / BACKUP`、诊断 `診断 / DIAGNOSTICS`、小说 `小説 / NOVEL`。本批建议：导入 `インポート / IMPORT`、详情 `詳細 / DETAIL`。
- **三态原语选择规则**：
  - 单一数据源、整页切换 → `StateBoundary`（loading/error/empty/ready + onRetry），参照 StatsPage/BackupPage/DiagnosticsPage
  - 多区块并存、部分加载、空态需自定义主行动 → 细粒度 `AsyncState`，参照 DownloadPage/NovelPage
  - 内容级空列表（ready 分支内的空面板）保留 `EmptyState`，不进页级状态机
- **FilterBar** 是筛选/工具条容器（非切换器）。tab 切换用 `FilterBar + SegmentControl` 组合（参照 DiscoveryPage）；单纯工具条（搜索框+按钮）直接 FilterBar。
- **ContentGrid 不适用**：需要固定列数 + 精确断点的 bento 网格时沿用自定义网格（Backup/Diagnostics/Novel 均未用 ContentGrid）。
- 沉浸子视图（阅读器、详情内部）不套 PageHeader；PageShell 用 `ariaLabel` 而非 `labelledBy`，避免标题 id 悬空。

### Step 3 · i18n

- store：`src/lib/stores/i18n.svelte.ts`，API 为 `i18n.t(key)`（响应式），zh/en 两个字典并排维护。
- 已有前缀：`app.*`、`menu.*`、`button.*`、`settings.*`、`discovery.*`、`stats.*`、`downloads.*`（含 `download.status.*`）、`backup.*`、`diagnostics.*`、`novel.*`。**新增 key 前先查重，既有 key 直接复用**。
- 本批新前缀：`platform_import.*`（不要用 `import.*`，避免与关键字及潜在 key 冲突）、`gamedetail.*`。
- **覆盖度要求**（大文件放宽）：页标题、区块标题、主按钮、空态/错误态文案必须接 `i18n.t()`；次要 label 可保持中文原文，但要在汇报中说明覆盖度。

### Step 4 · 契约测试

每页新增 `src/lib/components/<name>-ui-v2-migration.test.ts`，用 `readFileSync` 断言源码结构（模板照抄任一已有契约测试），至少断言：

- 使用 `<PageShell`、`<PageHeader`、三态原语（`StateBoundary` 或 `AsyncState`）
- 引用 i18n（`i18n.t(` 或对应前缀 key）
- 页面关键结构存在（如 PlatformImportPage 的 Steam 成就同步面板、GameDetailPage 的 Hero/档案区）
- 负向断言（可选）：已删除的死链元素不再出现

---

## 3. 两个目标页面的具体指令

### 3.1 PlatformImportPage.svelte（约 1446 行）

- 内含 Steam / Epic / 模拟器导入流程 + **最近刚挪进来的 Steam 成就同步面板**（位于 Steam 账号配置区，API Key/SteamID 字段附近）——**必须完整保留**。
- 大文件重点排查：互斥分支吞错、首载假空态、未消费 state。导入流程（扫描、登录、OAuth、结果回填）逻辑零改动。
- 可拆子组件到 `src/lib/components/platform-import/`（如 Steam/Epic/模拟器各一块），但非强制；拆分时不改变事件与数据流。
- i18n 用 `platform_import.*` 前缀。

### 3.2 GameDetailPage.svelte（约 930 行）

- **0.13.7 刚定稿的全宽电影档案页**：Hero、视觉接触表、封面、评分、简介、元数据、截图、存档、会话信息。**版式像素级不动，只替换页面骨架与三态**。
- 已存在 1 处 ui-v2 引用（先 Read 找到它，沿用其导入方式）。
- 游戏启动、刮削对话框、存档面板（SavePanel）等挂载逻辑零改动。
- i18n 用 `gamedetail.*` 前缀。

---

## 4. 已知坑（来自前 7 页的教训）

1. `report`/`error` 互斥分支会吞错误（DiagnosticsPage 实例）——大文件必查。
2. 首载 `[]` 会闪假空态（DownloadPage、DiscoveryPage 推荐区实例）——加 initialLoading/facetLoading 门闩。
3. GSAP/手写动画只守卫 media query 不够——必须补 `[data-motion="reduce"]` 双信号。
4. aura 旧主题类（`aura-page`/`aura-inset`/`data-aura-echo` 等）随迁移退役，但 **app.css 本体不要动**——其他未迁移页面仍在消费。
5. `SakuraParticles` 粒子层无 reduced-motion 处理，迁移后不再引用（组件本体保留在仓库）。
6. i18n key 不要重复建——先 grep 字典（真实案例：`downloads.preflight_probe_timeout` 并不存在，实为 `preflight_ok/required/available/quota/fail`）。
7. svelte-check 对 unused selector 会报警——清理死 CSS 后 `:global()` 包裹的规则要同步核对。
8. DownloadPage 有两个原版就存在的未消费残留（`animeClearFinishedDownloads` import、`animeDoneCount` derived），当时未动；本批若遇到同类"历史残留但 check 不报警"的项，**只记录不删**，列入汇报遗留。

---

## 5. 闸门与验收（全部实跑，缺一不可）

```bash
npm run check          # 必须 0 errors, 0 warnings
npm run test:unit      # 基线 101 文件/467 用例；完成后应全过且用例数 ≥ 467 + 新增契约数
npx vitest run src/lib/components/platform-import-ui-v2-migration.test.ts src/lib/components/gamedetail-ui-v2-migration.test.ts
npm run build          # 构建成功（chunk 体积警告可接受）
```

附加（如时间允许）：`npm run verify:versions`、`npm run verify:commands` 应通过（本方案不动 Rust，理论上无影响）。

---

## 6. 批次 3 完成后的收尾清单（可选，按优先级）

1. **app.css aura 旧主题退役**：确认所有页面迁完后，审计并删除 `src/app.css` 中无消费的 `.aura-*` 规则。
2. **死代码终审**：孤儿 i18n key、`src/lib/api` 中已无调用方的 `importWallpaper`、`ColorMode` 类型（设置页删减后遗孤）。
3. **i18n 启动对齐**：`i18n.svelte.ts` 启动只读 localStorage，未与后端 settings 的 `language` 字段做兜底对齐（跨设备首次登录短暂不一致）。
4. **locale 联动**：Novel 的 `formatDate`、Diagnostics 日志井时间戳固定 `zh-CN`，如需跟随界面语言再统一处理。
5. **DownloadPage 残留**：`animeClearFinishedDownloads` import 与 `animeDoneCount` derived 未消费（见第 4 节第 8 条）。
6. 若仓库中还存在其他未接入页面（如首页/合集页），按第 2 节 SOP 同法处理，先核查再迁移。

---

## 7. 执行完毕后的汇报格式

1. 两页各自的死链路核查结论（逐项表格）
2. 修改/新增文件清单（路径）
3. 设计落点与 i18n 覆盖度说明
4. 五道闸门的实际输出摘要
5. 遗留问题清单

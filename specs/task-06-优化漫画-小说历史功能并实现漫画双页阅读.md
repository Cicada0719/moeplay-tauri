# 子任务 6 开发规格说明：优化漫画/小说历史功能并实现漫画双页阅读

## 1. 任务目标

在子任务 4 交付的历史数据模型 v2（SQLite）基础上，完成漫画/小说历史的**精确续读**（漫画精确到「第几话第几页」，小说精确到「第几章滚动百分比」）与**历史列表增强**（类型筛选 / 排序 / 搜索 / 删除管理），并在漫画阅读器中实现**单页 / 双页（左右分屏）阅读模式**，支持 LTR / RTL 翻页方向、跨页大图自动识别与窄窗口降级提示。对应 PRD 功能 **FR-10、FR-11**。

---

## 2. 上下文与约束

### 2.1 PRD 关联

- **FR-10**（漫画/小说历史功能优化）：精确续读、Tab 筛选、搜索、单条/批量/按类型删除、墓碑同步联动。
- **FR-11**（漫画双页阅读模式）：单/双页切换、LTR/RTL、跨页大图独占整屏、窗口 <800px 提示、模式按漫画持久化。
- 非功能需求 §4.1：历史列表 1 万条虚拟滚动流畅、首屏渲染 ≤300ms；双页模式 ±2 屏预取、图片内存缓存 ≤200MB（LRU 淘汰）；双页 ≥50 FPS、翻页延迟 <100ms。
- §5.4 / 风险 R8：**页索引以单页为原子单位存储（与渲染模式无关），双页仅为视图层配对**——这是本任务最重要的架构约束。

### 2.2 技术栈与现有结构

- Tauri 2 + Svelte（沿用现有 Svelte 版本与组件库，不新增 UI 框架）。
- 前端代码位于 `src/lib/`；本任务涉及模块：
  - `src/lib/components/history/`（历史页组件目录）
  - `src/lib/components/history/HistoryList.svelte`
  - `src/lib/components/reader/`（阅读器组件目录）
  - `src/lib/components/reader/ComicReader.svelte`
  - `src/lib/reader/dualPage.ts`（**新建**，双页配对纯函数模块）
  - `src/lib/stores/readerSettings.ts`（**新建**，阅读器设置持久化 store）
- 本地存储已由子任务 4 迁移至 SQLite，历史表结构见 §5.3（含 `page_index`、`scroll_pct`、`chapter_id`、`deleted` 墓碑字段）。
- 历史数据读写统一通过 Tauri command 调用 Rust 侧，禁止前端直接访问 SQLite 文件。

### 2.3 约束

- 页索引、滚动百分比等位置字段**必须与渲染模式解耦**：无论单页/双页/LTR/RTL，`history.page_index` 始终存「单页序号」。
- 双页配对逻辑必须实现为**纯函数**（`dualPage.ts`），不依赖 DOM / Svelte，保证可单测。
- 不改动播放器（番剧）相关代码；番剧历史 Tab 仅做**展示与筛选复用**，续播逻辑若已存在则复用，不存在则本任务仅保证数据结构兼容、不实现番剧续播（属于任务 3 范围）。
- 新增依赖需控制在最小集：虚拟滚动如现有代码无方案，允许引入 `svelte-virtuallists` 或自实现（优先自实现轻量版，见 §4 步骤 3）。

---

## 3. 输入 / 输出

### 3.1 数据结构定义

#### 3.1.1 历史记录项（与子任务 4 对齐，TypeScript 侧）

```typescript
// src/lib/history/types.ts（若子任务4已建立则复用其定义，本任务不得重定义冲突）
export type ContentType = 'anime' | 'manga' | 'novel';

export interface HistoryItem {
  id: string;             // uuid
  contentId: string;
  contentType: ContentType;
  title: string;
  cover: string | null;
  sourceId: string;
  chapterId: string | null;
  chapterTitle: string | null;
  pageIndex: number;      // 漫画：单页序号（原子单位）；番剧：集内位置
  positionSec: number;    // 番剧播放秒数（本任务不使用）
  scrollPct: number;      // 小说滚动百分比 0~1
  updatedAt: number;      // Unix 秒
  deviceId: string;
  deleted: 0 | 1;         // 墓碑标记；列表查询默认过滤 deleted=1
}
```

#### 3.1.2 阅读器设置（本任务新增）

```typescript
// src/lib/stores/readerSettings.ts
export type PageMode = 'single' | 'dual';
export type ReadingDirection = 'ltr' | 'rtl';

export interface ReaderSettings {
  pageMode: PageMode;
  direction: ReadingDirection;   // 默认 'rtl'（日漫）
  forceNarrowDual: boolean;      // 窗口 <800px 时用户选择"仍要双页"
}

// 持久化键设计
// 全局默认：localStorage["reader:settings:global"] -> ReaderSettings
// 单漫画覆盖：localStorage["reader:settings:manga:{contentId}"] -> Partial<ReaderSettings>
```

> 说明：模式持久化要求「按漫画」（验收第 5 条），采用「全局默认 + 单漫画覆盖」两级存储。读取优先级：单漫画覆盖 > 全局默认 > 内置默认值 `{ pageMode:'single', direction:'rtl', forceNarrowDual:false }`。

#### 3.1.3 双页配对（纯函数模块输入输出）

```typescript
// src/lib/reader/dualPage.ts

/** 页面元信息（仅需宽高，由图片预加载后填充；未知时按普通页处理） */
export interface PageMeta {
  index: number;        // 单页序号，0-based，全局原子单位
  width?: number;
  height?: number;
}

/** 一屏（一次渲染单元）包含 1 或 2 个页面 */
export interface Screen {
  pageIndexes: number[];        // 长度 1（跨页大图独占）或 2
  anchorIndex: number;          // 该屏的"锚定页"，用于历史记录与恢复定位（取 pageIndexes 最小值）
}

/** 是否跨页大图：宽 > 高 × 1.5 */
export function isSpreadImage(width: number, height: number): boolean;

/**
 * 计算双页模式下的屏幕序列（纯函数）。
 * 规则：
 *  - 跨页大图独占一屏；
 *  - 其余页两两配对；
 *  - 配对不受方向（LTR/RTL）影响，方向仅影响渲染顺序与翻页映射（R8 约束）。
 */
export function buildScreens(pages: PageMeta[]): Screen[];

/**
 * 给定当前单页页索引，返回其所在屏幕的下标；用于从历史恢复定位。
 */
export function screenIndexOfPage(screens: Screen[], pageIndex: number): number;

/**
 * 翻页映射。
 * @param screens   屏幕序列
 * @param current   当前屏幕下标
 * @param intent    用户意图：'forward'(内容前进) | 'backward'(内容后退)
 * @param mode      单/双页
 * @param direction 阅读方向（仅影响 UI 层的按键/点击区域映射，本函数不涉及）
 * @returns 目标屏幕下标（越界时返回 clamp 后的边界值）
 */
export function nextScreen(screens: Screen[], current: number, intent: 'forward' | 'backward'): number;

/**
 * 将"物理方向输入"（点击左/右区域、按 ←/→ 键）翻译为内容意图。
 * RTL：右区域 / → 键 = forward；左区域 / ← 键 = backward。
 * LTR：左区域 / ← 键 = backward；右区域 / → 键 = forward。
 */
export function intentFromInput(
  physical: 'left' | 'right',
  direction: ReadingDirection
): 'forward' | 'backward';
```

#### 3.1.4 历史列表查询参数与结果

```typescript
// src/lib/history/historyApi.ts（本任务新增，封装 Tauri invoke）
export interface HistoryQuery {
  contentType?: ContentType;   // 不传 = 全部
  keyword?: string;            // 标题关键词，前端本地过滤（见 §4 步骤 4 决策）
  limit?: number;
  offset?: number;
  includeDeleted?: boolean;    // 默认 false
}

export interface HistoryPage {
  items: HistoryItem[];
  total: number;               // 满足筛选条件的总数（用于虚拟滚动高度计算）
}
```

### 3.2 Tauri Command 接口（依赖子任务 4，若已存在则对齐其签名）

| Command | 参数 | 返回 | 说明 |
|---|---|---|---|
| `history_query` | `query: HistoryQuery` | `HistoryPage` | 按类型筛选 + `updated_at DESC` 排序，过滤墓碑 |
| `history_upsert` | `item: HistoryItem` | `void` | 按 `content_id + source_id` 存在则更新位置字段与 `updated_at` |
| `history_delete` | `ids: string[]` | `void` | **软删除**：置 `deleted=1` 并更新 `updated_at`（墓碑机制，FR-10 验收第 4 条） |
| `history_clear_by_type` | `contentType: ContentType` | `void` | 该类型全部软删除 |

> 若子任务 4 已提供等价 command 但命名不同，以子任务 4 实际签名为准并在 `historyApi.ts` 内做适配，**不得修改 Rust 侧已有实现**。

### 3.3 组件接口

```svelte
<!-- src/lib/components/history/HistoryList.svelte -->
<script lang="ts">
  // Props
  export let initialTab: ContentType | 'all' = 'all';
  // Events (Svelte dispatcher)
  // on:resume  { item: HistoryItem }        —— 点击条目，请求续读/续播
  // on:deleted { ids: string[] }            —— 删除完成（供外层 Toast/统计）
</script>
```

```svelte
<!-- src/lib/components/reader/ComicReader.svelte -->
<script lang="ts">
  // Props
  export let contentId: string;
  export let sourceId: string;
  export let chapterId: string;
  export let chapterTitle: string;
  export let pages: string[];            // 图片 URL 列表（单页序）
  export let initialPageIndex?: number;  // 从历史进入时传入；默认 0
  // 内部职责：工具栏（单/双页切换、LTR/RTL 切换）、屏幕渲染、预取、
  //          位置上报（debounce 2s + 组件销毁时 flush 写 history_upsert）
</script>
```

```svelte
<!-- src/lib/components/reader/NovelReader.svelte（若已存在则改造） -->
<!-- 新增职责：scroll 事件 debounce 500ms 计算 scrollPct = scrollTop/(scrollHeight-clientHeight)，
     写入历史；initialScrollPct prop 进入后恢复（渲染完成后执行 scrollTo） -->
```

---

## 4. 实现步骤（按顺序执行）

### 步骤 1：历史 API 封装层

**文件：`src/lib/history/historyApi.ts`（新建）**

1. 定义 §3.1.1 / §3.1.4 的类型（若 `types.ts` 已由子任务 4 建立则 import 复用）。
2. 实现函数：
   - `queryHistory(q: HistoryQuery): Promise<HistoryPage>` → `invoke('history_query', { query: q })`。
   - `upsertHistory(item: HistoryItem): Promise<void>`。
   - `deleteHistory(ids: string[]): Promise<void>`（内部调用 `history_delete`，软删除）。
   - `clearHistoryByType(t: ContentType): Promise<void>`。
3. 所有函数包裹 try/catch，异常时抛出带中文语境的 `Error('历史记录读取失败')` 等，供 UI 统一 Toast。
4. 导出一个内存缓存 `historyCache: Writable<HistoryItem[] | null>`（Svelte store）：查询成功后写入；删除/清空后同步剔除缓存项，避免二次拉取。

### 步骤 2：阅读器设置 Store

**文件：`src/lib/stores/readerSettings.ts`（新建）**

1. 定义 §3.1.2 类型与默认值常量 `DEFAULT_SETTINGS`。
2. 实现：
   - `loadSettings(contentId?: string): ReaderSettings` —— 读 localStorage，合并「默认 ← 全局 ← 单漫画覆盖」，JSON 解析失败回退默认并 console.warn。
   - `saveSettings(contentId: string | null, patch: Partial<ReaderSettings>): void` —— `contentId` 为 null 时写全局键，否则写 `reader:settings:manga:{contentId}` 键（merge 语义）。
   - `createReaderSettingsStore(contentId: string)` —— 返回 Svelte `writable<ReaderSettings>`，其 `set/update` 自动调用 `saveSettings(contentId, …)` 实现持久化。
3. **不允许**把设置写入 SQLite（轻量偏好走 localStorage，避免阻塞）。

### 步骤 3：双页配对纯函数模块

**文件：`src/lib/reader/dualPage.ts`（新建）**

1. 按 §3.1.3 签名实现全部 5 个函数。核心逻辑要点：
   - `isSpreadImage`：`width > height * 1.5` 返回 true；`width/height` 缺失（未加载）返回 false。
   - `buildScreens`：遍历 `pages`，遇跨页图生成单元素 Screen；否则累积两个普通页生成一个 Screen；最后一页落单时生成单元素 Screen。**配对与 direction 无关**。
   - `screenIndexOfPage`：线性查找 `pageIndexes.includes(pageIndex)`，找不到抛 `RangeError`。
   - `nextScreen`：`forward → current+1`，`backward → current-1`，clamp 到 `[0, screens.length-1]`。
   - `intentFromInput`：严格按 §3.1.3 映射表实现（RTL 与 LTR 互为镜像）。
2. 本文件**零依赖**（不 import svelte / DOM API），保证可 vitest 直测。
3. 图片宽高探测辅助（可放本文件尾部或 `src/lib/reader/imageMeta.ts`）：
   - `probeImageSize(url: string): Promise<{width:number;height:number}>`，用 `new Image()` 加载，结果存入模块级 `Map<string, {width,height}>` 缓存，供 `buildScreens` 前的 PageMeta 装配使用。

### 步骤 4：历史列表页

**文件：`src/lib/components/history/HistoryList.svelte`（新建或重写）**

1. **布局结构**：
   ```
   <header>
     [Tab: 全部 | 番剧 | 漫画 | 小说]   [搜索框(防抖150ms)]   [批量管理模式按钮]
   </header>
   <div class="virtual-list">  <!-- 固定行高 72px，虚拟滚动 -->
     <HistoryRow />            <!-- 封面缩略图、标题、章节信息、相对时间、来源、删除checkbox(批量模式) -->
   </div>
   <footer>批量模式时显示：全选 | 删除所选 | 清空当前类型</footer>
   ```
2. **数据流**：切换 Tab / 首次挂载 → `queryHistory({contentType, limit: 全部})` 拉取该类型全量（1 万条级 JSON 在本地 IPC 下可接受）→ 存入本地数组；搜索关键词在前端对该数组 `filter`（`title.includes(keyword)`，均转小写），**200ms 内完成**（验收要求），匹配子串用 `<mark>` 高亮（封装 `highlight(text, keyword)` 工具函数返回 `{pre, hit, post}` 片段数组以避免 XSS 的 `{@html}`）。
3. **虚拟滚动**：自实现轻量版——容器 `on:scroll` 计算 `startIndex = floor(scrollTop / ROW_H)`，渲染 `startIndex-5 ~ startIndex+visibleCount+5`，上下用 `padding` 撑高度。**禁止**一次性渲染 1 万行 DOM。若项目已有虚拟滚动组件则复用。
4. **排序**：统一 `updatedAt DESC`（SQL 层已排好，前端不再重排）。
5. **删除交互**：
   - 单条：行内垃圾桶图标 → 确认 Popover → `deleteHistory([id])` → 本地剔除 + Toast「已删除」。
   - 批量：进入管理模式出现 checkbox；「删除所选」二次确认（Modal：「将删除 N 条记录，同步后其他设备也会删除」）。
   - 按类型清空：footer 按钮 → Modal 二次确认 → `clearHistoryByType(currentTab)`（`all` Tab 下该按钮禁用或逐个类型调用，实现选择前者并禁用）。
6. **续读/续播**：行点击 `dispatch('resume', { item })`。父页面路由逻辑（见步骤 7）。
7. **相对时间**：`formatRelativeTime(updatedAt)` 工具（刚刚 / n 分钟前 / n 小时前 / n 天前 / 超 7 天显示日期），放 `src/lib/utils/time.ts`（已有则复用）。
8. **空态**：无记录显示插画占位 + 「暂无记录」；搜索无结果显示「未找到匹配记录」。

### 步骤 5：漫画阅读器改造（双页模式）

**文件：`src/lib/components/reader/ComicReader.svelte`（重写渲染与交互层）**

1. **状态装配**（`onMount`）：
   - `createReaderSettingsStore(contentId)` 加载模式与方向。
   - 预探测所有页尺寸：`pages.map(probeImageSize)`（`Promise.allSettled`，不阻塞首屏；未探测到的页先按普通页配对，**尺寸就绪后重建 screens**——重建时保持当前 `pageIndex` 所在屏可见，用 `screenIndexOfPage` 重定位，避免跳动）。
   - `screens = buildScreens(pageMetas)`（单页模式下每页即一屏，可复用同一代码路径：`buildScreens` 退化为每页一屏）。
2. **渲染**：
   - 单页模式：一屏一图，`object-fit: contain`。
   - 双页模式：当前 Screen 的 `pageIndexes` 渲染为横向 flex 两图，**RTL 时容器 `flex-direction: row-reverse`**（使第一页在右）；每图 `max-width: 50%; max-height: 100%; object-fit: contain`（保证不裁剪，满足窄窗口强制双页验收）。
   - 跨页图独占屏：单图 `max-width: 100%`。
   - 切换动画：CSS `transition: opacity 200ms`（<300ms 验收）。
3. **翻页交互**：
   - 点击区域：屏幕左 35% / 右 35% 为翻页热区，中部 30% 呼出工具栏。点击热区 → `intentFromInput(physical, direction)` → `nextScreen(...)`。
   - 键盘：`on:keydown`（`window` 级，`onDestroy` 移除）→ ArrowLeft/ArrowRight 同上映射；空格 = forward。
   - 工具栏按钮：上一屏/下一屏（语义即 forward/backward，图标随 direction 变化）。
4. **工具栏**：
   - 「单页 / 双页」切换按钮、「LTR / RTL」切换按钮、页码指示「{当前屏页范围} / {总页数}」（如 `7-8 / 120`）。
   - 双页切换时窗口宽度检测：`window.innerWidth < 800` 且 `!forceNarrowDual` → 弹出确认条「窗口过窄，建议单页阅读」+ [仍要双页] [取消]；选「仍要双页」→ `saveSettings(contentId, { forceNarrowDual: true, pageMode:'dual' })`。
   - 模式/方向变更即写 store（自动持久化到单漫画键）。
5. **位置上报（精确到页）**：
   - 当前屏变化后，取 `screens[current].anchorIndex` 作为 `pageIndex`，debounce 2s 调 `upsertHistory`（字段：`contentType:'manga'`，`pageIndex`、`chapterId/chapterTitle`、`cover`、`title` 等，`updatedAt: Date.now()/1000`）。
   - `onDestroy` 时立即 flush 一次（取消防抖直接写），保证「退出即记录」。
6. **预取与内存**：
   - 维护 `preloadQueue`：当前屏下标 ±2 屏对应的所有 `pageIndexes` 的 URL，用 `new Image()` 预取。
   - LRU：模块级 `Map<url, {lastUsed}>`，估算占用（`width*height*4` 字节），累计 >200MB 时淘汰最久未用项的缓存引用（`img.src=''` 释放解码位图依赖浏览器 GC，记录元数据即可）。
7. **从历史恢复**：`initialPageIndex` 传入时，`current = screenIndexOfPage(screens, initialPageIndex)`；screens 因尺寸探测重建后需重算一次。保证「第 12 话第 7 页 → 恢复 ±0 页」。

### 步骤 6：小说阅读器滚动位置

**文件：`src/lib/components/reader/NovelReader.svelte`（改造，若不存在则新建基础版）**

1. 新增 props：`initialScrollPct?: number`。
2. 滚动容器 `on:scroll` debounce 500ms：计算 `scrollPct = scrollTop / (scrollHeight - clientHeight)`（除零保护：分母 ≤0 时记 0），写 `upsertHistory`（`contentType:'novel'`、`scrollPct`）。
3. 挂载且章节内容渲染完成后（`tick()` 后 + 字体/图片 load 的 `requestAnimationFrame` 两帧延迟），执行 `container.scrollTo({ top: initialScrollPct * (scrollHeight - clientHeight) })`。
4. `onDestroy` flush 上报一次。

### 步骤 7：路由 / 续读入口接线

**文件：`src/routes/history/+page.svelte`（或项目实际的历史页路由文件）**

1. 挂载 `<HistoryList on:resume={handleResume} />`。
2. `handleResume(e)`：
   - `manga` → 跳转漫画阅读器路由，携带 `{ contentId, sourceId, chapterId, initialPageIndex: item.pageIndex }`。
   - `novel` → 跳转小说阅读器路由，携带 `{ ..., initialScrollPct: item.scrollPct }`。
   - `anime` → 若已有番剧播放页路由则携带 `positionSec` 跳转；若无（任务 3 范围），Toast「该记录为番剧，请从播放页进入」占位，**不在本任务实现番剧续播**。
3. 漫画/小说阅读器页面从路由参数读取 initial 值并透传给组件 props。

### 步骤 8：删除→同步联动检查

- 确认步骤 1 中所有删除均走软删除 command（墓碑），在前端注释中标注「同步（任务 5）将依据墓碑传播删除」。本任务不实现同步逻辑本身，但**禁止**新增任何物理删除路径。

---

## 5. 依赖的外部接口

| 依赖 | 来源 | 接口假设 | 对接方式 |
|---|---|---|---|
| 历史数据 v2 Tauri commands | 子任务 4（FR-08） | §3.2 表格中的 4 个 command 已可用，含墓碑软删除语义 | `historyApi.ts` 集中封装 invoke；若实际签名不同，仅改适配层 |
| `HistoryItem` 类型 | 子任务 4 | 字段如 §3.1.1 | import 复用，不重定义 |
| 漫画/小说阅读器现有路由与数据获取（章节图片列表、章节文本） | 现有代码 | `pages: string[]` 可通过现有章节加载逻辑获得 | ComicReader 保持现有章节加载入口不变，仅替换渲染层 |
| 墓碑删除传播 | 子任务 5（FR-09，并行开发） | 同步器读取 `deleted=1` 记录 | 本任务只保证软删除写库正确，通过 command 层契约对接 |
| Toast / Modal 等基础 UI 组件 | 现有组件库 | 存在全局 Toast 与确认 Modal | 复用；若无确认 Modal，在 `src/lib/components/common/ConfirmModal.svelte` 新建通用组件 |

---

## 6. 测试要求

### 6.1 单元测试（vitest，`src/lib/reader/__tests__/dualPage.test.ts` 等）

**dualPage.ts（纯函数，覆盖率要求 ≥90%）**

| # | 用例 | 类型 |
|---|---|---|
| U1 | `isSpreadImage(2000, 1000)` → true；`(1000, 2000)` → false；`(1500, 1000)`（恰好 1.5）→ false | 边界 |
| U2 | 10 个普通页 → 5 屏，每屏 2 页，anchorIndex 为 `[0,2,4,6,8]` | 正常 |
| U3 | 9 个普通页 → 5 屏，末屏单页 | 边界 |
| U4 | 页序列 `[普通, 跨页, 普通, 普通]` → 3 屏：`[0], [1], [2,3]` | 正常 |
| U5 | 连续两张跨页图各自独占一屏 | 边界 |
| U6 | `screenIndexOfPage(screens, 7)` 命中配对屏；传入越界 index 抛 `RangeError` | 正常+异常 |
| U7 | `nextScreen` 在 0 处 backward 返回 0；末屏 forward 返回末屏（clamp） | 边界 |
| U8 | `intentFromInput('right','rtl')` → `'forward'`；`('right','ltr')` → `'backward'`；left 两侧镜像（4 组合全测） | 正常 |
| U9 | 空 pages 数组 → `buildScreens` 返回 `[]`；`screenIndexOfPage([], 0)` 抛错 | 异常 |

**readerSettings.ts**

| # | 用例 |
|---|---|
| U10 | 无存储时返回默认值 `{single, rtl, false}` |
| U11 | 单漫画覆盖优先于全局：全局 dual + 漫画 A 存 single → 读漫画 A 得 single，读漫画 B 得 dual |
| U12 | localStorage 中 JSON 损坏 → 回退默认并 warn，不抛异常 |
| U13 | store `update` 后对应键被写入（mock localStorage 断言） |

**historyApi.ts**（mock `invoke`）

| # | 用例 |
|---|---|
| U14 | `queryHistory` 透传参数并返回分页结构；invoke 抛错时转为中文 Error |
| U15 | `deleteHistory` 后缓存中对应条目被剔除 |

### 6.2 组件测试（vitest + @testing-library/svelte，或现有测试栈）

| # | 用例 | 类型 |
|---|---|---|
| C1 | HistoryList：渲染 3 条记录，Tab 切到「漫画」仅显示漫画项 | 正常 |
| C2 | HistoryList：输入关键词 150ms 防抖后过滤，命中子串被 `<mark>` 包裹；无结果显空态文案 | 正常 |
| C3 | HistoryList：单条删除 → 确认后调用 `deleteHistory([id])` 且行消失 | 正常 |
| C4 | HistoryList：批量模式勾选 2 条 → 删除 → 二次确认文案含「N=2」与同步提示 | 正常 |
| C5 | HistoryList：「清空当前类型」在 `all` Tab 下禁用；在「漫画」Tab 下调用 `clearHistoryByType('manga')` | 边界 |
| C6 | HistoryList：构造 10000 条数据，断言渲染的 DOM 行数 < 50（虚拟滚动生效） | 性能 |
| C7 | ComicReader 双页 RTL：点击右侧热区 → 前进一屏（两页）；按 `→` 键同效；点击左侧 / `←` 后退 | 正常 |
| C8 | ComicReader：含跨页图时该屏仅渲染 1 个 `<img>`，页码指示总数不变 | 正常 |
| C9 | ComicReader：`window.innerWidth=700` 切双页 → 出现「窗口过窄」提示；点「仍要双页」后双页生效且图片 `object-fit: contain` | 边界 |
| C10 | ComicReader：`initialPageIndex=6`（奇数屏锚定场景）进入 → 定位到含第 6 页的屏，不串页（R8 回归） | 正常 |
| C11 | ComicReader：`onDestroy` 触发 flush，调用一次 `upsertHistory` 且 `pageIndex` 为当前 anchor | 正常 |
| C12 | NovelReader：mock scrollHeight/clientHeight，scroll 至 45% → 防抖后 `upsertHistory` 收到 `scrollPct≈0.45`；`initialScrollPct=0.45` 进入后 `scrollTo` 参数正确（±5% 容差断言） | 正常 |

### 6.3 集成 / 手动验收（对照 acceptance）

1. 漫画读至第 12 话第 7 页 → 退出 → 历史列表点击 → 恢复到第 12 话第 7 页（±0 页）。
2. 小说读至第 30 章 45% → 退出 → 历史进入 → 恢复 45%（±5%）。
3. 双页模式退出再进入：页索引为奇数起始时配对正确不串页。
4. 1 万条历史首屏渲染 ≤300ms（Performance API 打点记录）。
5. 单↔双页切换动画 <300ms；模式按漫画持久化（开另一部漫画不受影响）。
6. 删除一条历史后检查 SQLite：`deleted=1` 且记录仍存在（墓碑），列表不可见。

---

## 7. 完成定义 Definition of Done

- [ ] `historyApi.ts`、`readerSettings.ts`、`dualPage.ts` 三个新模块按 §3 签名实现完毕，`dualPage.ts` 零 DOM/Svelte 依赖。
- [ ] `HistoryList.svelte` 实现 Tab 筛选 / 关键词搜索（防抖+高亮，200ms 内）/ 最近更新排序 / 虚拟滚动（1 万条 DOM 行数受控，首屏 ≤300ms）/ 单条删除 / 批量删除 / 按类型清空（全部走软删除）。
- [ ] `ComicReader.svelte` 实现单/双页切换（<300ms 动画）、LTR/RTL（含 RTL 下点击/按键方向映射正确）、跨页大图独占屏、<800px 提示与强制开启、模式按漫画持久化、±2 屏预取 + 200MB LRU、位置 debounce 上报 + destroy flush。
- [ ] `NovelReader.svelte` 实现 scrollPct 上报（500ms 防抖）与恢复（±5%）。
- [ ] 历史点击续读路由接线完成：漫画 ±0 页、小说 ±5% 验收通过。
- [ ] §6.1 全部单测（U1~U15）通过，dualPage.ts 覆盖率 ≥90%；§6.2 组件测试 C1~C12 通过。
- [ ] §6.3 手动验收 6 项全部通过并录屏/截图存档。
- [ ] 页索引始终以单页原子单位存储（代码评审确认无双页模式写入渲染层页号的代码路径）。
- [ ] 无新增 `console.log` 调试残留；ESLint/类型检查（`svelte-check`）零 error。

---

## 8. 禁止修改清单

| 路径 / 范围 | 原因 |
|---|---|
| `src-tauri/`（Rust 侧全部，含 migrations、commands 实现） | 属子任务 4 产出；本任务仅消费其 command 接口 |
| `src/lib/components/player/` 及播放器相关文件 | 属子任务 3（FR-06/07）范围 |
| 规则引擎与源相关代码（`src/lib/rules/`、`resources/rules/`） | 属子任务 1/2 范围 |
| 同步相关代码（WebDAV 客户端、同步设置页） | 属子任务 5 范围；本任务仅保证墓碑删除契约 |
| `release.yml` 及 CI workflow | 本期不改动 CI 主流程 |
| 番剧历史的写入逻辑（position_sec 上报） | 若已存在不得改动；番剧续播不实现 |
| `package.json` / `Cargo.toml` | 不得新增依赖（虚拟滚动与动画均自实现/CSS 完成）；如确需新增须经评审 |
| 现有全局样式变量 / 主题文件 | 避免影响其他并行任务 UI |
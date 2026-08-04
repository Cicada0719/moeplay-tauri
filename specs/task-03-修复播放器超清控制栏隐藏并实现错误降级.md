# 子任务 3 开发规格说明：修复播放器超清控制栏隐藏并实现错误降级

## 一、任务目标

重构播放器控制栏隐藏逻辑，收敛为单一的 `useIdleTimer` Svelte action 管理空闲状态，根治"切换超清（4K）画质后控制栏无法自动隐藏"的 bug（根因：画质切换重建播放器容器导致旧定时器/事件监听悬空）。同时实现结构化播放错误降级 UI（ErrorOverlay），在播放失败时提供"重试 / 切换源 / 复制日志"等可操作入口，连续重试失败自动弹出按健康度排序的源推荐列表。覆盖 PRD 中 **FR-06**（P0）与 **FR-07**（P1）。

## 二、上下文与约束

- **关联 PRD 条目**：FR-06（超清画质控制栏自动隐藏修复，P0）、FR-07（播放器错误降级提示，P1）；联动 FR-02（源切换状态保持，任务 1 产出）。
- **技术栈**：Tauri 2 + Svelte（4/5，沿用现有），前端播放器为现有 video.js 或原生 `<video>`，**不得更换播放器库**。若根因在第三方库内部，按 PRD 风险 R7 在 wrapper 层打补丁。
- **现有代码结构（假设，编码代理需先核实）**：
  - `src/lib/components/player/Player.svelte` — 播放器主容器（负责 video 元素生命周期、画质切换）
  - `src/lib/components/player/Controls.svelte` — 底部控制栏（含选集/画质下拉菜单）
  - `src/lib/stores/player.ts` — 播放器状态 store（当前画质、播放/暂停、进度等）
  - 画质切换当前实现为**销毁重建** video 元素或播放器实例，导致事件监听与 `setTimeout` 定时器悬空——这是本次要消除的架构问题。
- **约束**：
  - 非功能需求：控制栏重新显示延迟 < 200ms；画质切换后控制栏行为与其余画质完全一致。
  - 不改动规则引擎、Rust 侧代码与数据层；源切换与健康度数据通过既有/约定的接口消费（见第五节）。
  - 所有新逻辑必须保证 `onDestroy` 无泄漏：定时器、事件监听全部清理。

## 三、输入 / 输出

### 3.1 新建：`useIdleTimer` Svelte Action（`src/lib/actions/idleTimer.ts`）

```typescript
export interface IdleTimerOptions {
  /** 空闲超时毫秒数，默认 3000 */
  timeout?: number;
  /** 每次 tick 前回调：返回 true 表示暂停隐藏（如下拉菜单展开、非全屏） */
  shouldPause?: () => boolean;
  /** 进入空闲态（应隐藏控制栏与鼠标指针） */
  onIdle: () => void;
  /** 退出空闲态（应显示控制栏），需在 200ms 内完成 */
  onActive: () => void;
}

/**
 * Svelte action，挂载到播放器全屏容器。
 * 监听容器内 mousemove / mousedown / wheel / touchstart 与 window 级 keydown。
 * action destroy 时必须清除全部监听器与定时器（本次 bug 修复的核心保证）。
 */
export function idleTimer(
  node: HTMLElement,
  options: IdleTimerOptions
): { update: (opts: IdleTimerOptions) => void; destroy: () => void };
```

行为规约：
- 任意活动事件触发：若当前为 idle 态则先调用 `onActive()`，再重置计时器。
- 计时器到点前检查 `shouldPause?.()`，为 `true` 则**不隐藏**并继续以 500ms 轮询等待（避免菜单展开时隐藏）。
- `update()` 用于动态替换 `shouldPause` 闭包（如下拉菜单开合状态变化）。

### 3.2 修改：播放器状态 Store（`src/lib/stores/player.ts`）

新增/调整以下导出：

```typescript
/** 控制栏可见性状态机：visible | idle（idle = 控制栏与指针隐藏） */
export const controlsVisible: Writable<boolean>;

/** 下拉菜单打开计数（选集/画质菜单各 +1/-1），> 0 时暂停隐藏计时 */
export const openMenuCount: Writable<number>;

/** 播放错误状态 */
export type PlayerErrorKind =
  | 'NETWORK'      // 网络错误/超时
  | 'PARSE_EMPTY'  // 解析结果为空
  | 'HTTP_FORBIDDEN' // 403/防盗链
  | 'HTTP_ERROR'   // 其他 4xx/5xx
  | 'MEDIA_DECODE';  // 解码失败

export interface PlayerError {
  kind: PlayerErrorKind;
  message: string;          // 用户可读描述
  detail?: string;          // 原始错误/堆栈，供复制日志
  httpStatus?: number;
  url?: string;
  occurredAt: number;       // Date.now()
}

export const playerError: Writable<PlayerError | null>;
export const retryCount: Writable<number>;          // 当前错误已重试次数
export const showSourceSuggest: Writable<boolean>;  // 连续失败 ≥2 次后置 true

/** 动作 */
export function reportPlayerError(err: PlayerError): void;   // 记录错误，重置 retryCount
export function retryPlayback(): Promise<void>;              // 重新加载当前地址，retryCount+1，≥2 次仍失败则 showSourceSuggest=true
export function clearPlayerError(): void;                    // 恢复播放/切换源成功后调用，重置 retryCount、showSourceSuggest
```

### 3.3 修改：`Player.svelte` 画质切换

```typescript
/** 切换画质：复用现有 <video> 元素，仅替换 source 并恢复播放位置 */
export async function switchQuality(quality: QualityOption): Promise<void>;
// 逻辑要点：
// 1. 记录 currentTime 与 paused 状态
// 2. video.src = newUrl（或 video.js 的 player.src()），不销毁元素/实例
// 3. loadedmetadata 后 seek 回原位置，恢复原播放状态
// 4. 全程不触碰 idleTimer action（因其绑定在稳定的全屏容器上）
```

### 3.4 新建：`ErrorOverlay.svelte`（`src/lib/components/player/ErrorOverlay.svelte`）

```typescript
export interface ErrorOverlayProps {
  error: PlayerError;
  retryCount: number;
  onRetry: () => void;
  onSwitchSource: () => void;   // 触发源切换流程（保留进度，走任务 1 接口）
  onCopyLog: () => void;        // 复制 error.detail + 环境信息到剪贴板
}
```

### 3.5 新建：`SourceSuggestSheet.svelte`（`src/lib/components/player/SourceSuggestSheet.svelte`）

源推荐列表弹层：按健康度排序（可用 > 未知 > 异常），点击条目触发带进度保持的源切换。

### 3.6 错误码映射表（`src/lib/player/errorMap.ts`，新建）

| 触发条件 | kind | 用户提示文案 |
|---|---|---|
| `video.error` MEDIA_ERR_NETWORK / fetch 超时 | NETWORK | "网络连接失败，请检查网络后重试" |
| 解析接口返回空地址 | PARSE_EMPTY | "该源解析失败，建议切换源" |
| HTTP 401/403 | HTTP_FORBIDDEN | "该源拒绝访问（防盗链），建议切换源" |
| 其他 4xx/5xx | HTTP_ERROR | "播放地址返回错误（{status}）" |
| MEDIA_ERR_DECODE / SRC_NOT_SUPPORTED | MEDIA_DECODE | "视频解码失败，建议切换源或画质" |

## 四、实现步骤

1. **根因确认 spike（0.5 天）**：在 `Player.svelte` 中定位画质切换代码路径，确认是否为销毁重建 video 元素/播放器实例；确认现有控制栏隐藏定时器与 `mousemove` 监听绑定在哪个 DOM 节点上。将结论以注释形式记录在 `Player.svelte` 头部。

2. **新建 `src/lib/actions/idleTimer.ts`**：按 §3.1 实现 `idleTimer` action。要点：
   - 监听器挂 `node`（mouse 系）与 `window`（keydown，注意 destroy 时移除）；
   - 使用单个 `setTimeout`，`resetTimer()` 统一重置；
   - `shouldPause` 轮询间隔 500ms；
   - 补充 `src/lib/actions/` 下无目录则创建，并在 `src/lib/actions/index.ts`（若无则新建）re-export。

3. **改造 `src/lib/stores/player.ts`**：按 §3.2 新增 `controlsVisible`、`openMenuCount`、`playerError`、`retryCount`、`showSourceSuggest` 及三个动作函数。`controlsVisible` 变化时同步切换播放器容器 CSS class `idle`（`cursor: none`，`Controls` 透明度 0 + `pointer-events: none`）。

4. **改造 `Player.svelte`**：
   - 全屏根容器（`<div class="player-root">`）挂载 `use:idleTimer={{ timeout: 3000, shouldPause, onIdle, onActive }}`；该容器在画质切换期间**不重建**（无 `{#key}` 包裹）。
   - `shouldPause` 实现为：`() => $openMenuCount > 0 || !$isFullscreen || $playerError !== null`（错误弹层显示时不隐藏控制栏，保证按钮可点）。
   - `onIdle`：`controlsVisible.set(false)`；`onActive`：`controlsVisible.set(true)`。
   - 实现 `switchQuality()`（§3.3）：复用 `<video>` 元素替换 source、seek 恢复进度；删除旧实现中"销毁重建播放器"的分支。若第三方播放器库强制重建实例，则在重建后的 `ready` 回调中**重新初始化**该实例相关监听，idleTimer 因绑定在稳定容器上无需重建。
   - 移除散落在各处的旧隐藏定时器/`mousemove` 监听代码（确认全部被 idleTimer 接管）。

5. **改造 `Controls.svelte`**：
   - 可见性改为消费 `controlsVisible` store：`<div class="controls" class:hidden={!$controlsVisible}>`，`.hidden` 用 `opacity + transform` 过渡（≤ 200ms），并设 `pointer-events: none`。
   - 选集下拉、画质下拉组件的 `onOpen`/`onClose` 分别调用 `openMenuCount.update(n => n+1)` / `n => Math.max(0, n-1)`；组件 `onDestroy` 时若菜单仍展开需补 `-1`，防泄漏导致永不隐藏。
   - 鼠标指针隐藏：在 `player-root.idle` CSS class 下设置 `cursor: none`。

6. **新建 `src/lib/player/errorMap.ts`**：实现 `classifyPlaybackError(raw: unknown, httpStatus?: number): PlayerError`，按 §3.6 映射。

7. **新建 `ErrorOverlay.svelte`**（§3.4）：
   - 全屏半透明遮罩，展示错误图标、错误类型文案（来自 errorMap）、`httpStatus`；
   - 三个按钮：「重试」（调 `onRetry`）、「切换源」（调 `onSwitchSource`，主按钮，PARSE_EMPTY/HTTP_FORBIDDEN 时默认聚焦）、「复制日志」（`navigator.clipboard.writeText`，成功 Toast）；
   - 出现时机：`playerError` 非空即渲染，须在错误上报后 2 秒内可见（上报与渲染同帧，无额外延迟）。

8. **新建 `SourceSuggestSheet.svelte`**：从源健康 store（见第五节）读取当前内容类型的源列表，按健康度排序渲染；点击条目调用源切换接口（携带当前 `contentId / chapterId / positionSec`）。

9. **接入错误上报链路**：在 `Player.svelte` 中：
   - 监听 `<video>` 的 `error` 事件与播放地址 fetch 的失败分支，调用 `reportPlayerError(classifyPlaybackError(...))`；
   - `retryPlayback()` 内部：重新请求当前播放地址并加载，成功则 `clearPlayerError()`，失败则 `retryCount+1`，当 `retryCount >= 2` 时 `showSourceSuggest.set(true)` 并渲染 `SourceSuggestSheet`；
   - 切换源成功回调中调用 `clearPlayerError()`。

10. **CSS 与细节**：`player-root` 增加 `.idle { cursor: none; }`；Controls 过渡动画 ≤ 200ms；确保 `SourceSuggestSheet` 与 `ErrorOverlay` 展开时 `openMenuCount` 同样 +1（暂停隐藏计时）。

11. **自测与回归**：按第六节测试清单执行；重点验证 5 次画质来回切换的回归场景。

## 五、依赖的外部接口

本任务 `depends_on: []`，但 FR-07 的"切换源"与"源推荐列表"需消费以下接口（由任务 1 / 任务 2 产出，当前以**接口假设**方式对接；若实际接口未就绪，编码代理需在本模块内以薄适配层隔离，禁止硬编码）：

```typescript
// 假设接口 1（任务 1，FR-02 源切换）：
// src/lib/services/sourceSwitch.ts
export interface SwitchSourceParams {
  contentId: string;
  chapterId?: string;      // 集/话/章标识
  positionSec?: number;    // 播放进度，切换后续播
  targetSourceId: string;
}
export function switchSource(params: SwitchSourceParams): Promise<void>;

// 假设接口 2（任务 2，FR-04 源健康状态）：
// src/lib/stores/sources.ts
export type SourceHealth = 'ok' | 'degraded' | 'unknown';
export interface SourceInfo { id: string; name: string; contentType: string; health: SourceHealth; }
export function getSourcesFor(contentType: string): SourceInfo[]; // 或对应 readable store
```

对接方式：`SourceSuggestSheet` 与 `ErrorOverlay` 的 `onSwitchSource` 仅通过上述接口调用；若接口暂缺，在 `src/lib/services/sourceSwitch.ts` 内实现一个 `// TODO(task-1)` 标注的桩（弹出"源切换能力待接入"Toast），保证本任务可独立交付与测试。

## 六、测试要求

### 6.1 单元测试（Vitest，`src/lib/actions/__tests__/idleTimer.test.ts` 等）

- [ ] `idleTimer`：挂载后 3s 无事件触发 `onIdle` 恰好一次（fake timers）。
- [ ] `idleTimer`：2.9s 时触发 `mousemove`，第 3s 不触发 `onIdle`，第 5.9s 触发。
- [ ] `idleTimer`：idle 态下 `mousemove` / `keydown` 先调 `onActive` 再重置计时；连续事件不重复调 `onActive`。
- [ ] `idleTimer`：`shouldPause()` 返回 true 时到点不触发 `onIdle`，轮询至返回 false 后正常隐藏。
- [ ] `idleTimer`：`destroy()` 后再等待 5s，`onIdle`/`onActive` 均不被调用（无泄漏）；`window` 的 keydown 监听已移除（spy 验证）。
- [ ] `classifyPlaybackError`：403 → HTTP_FORBIDDEN；空地址 → PARSE_EMPTY；MEDIA_ERR_NETWORK → NETWORK；未知错误 → HTTP_ERROR 兜底。
- [ ] store：`reportPlayerError` 重置 `retryCount` 为 0；`retryPlayback` 连续 2 次失败后 `showSourceSuggest === true`；`clearPlayerError` 清空全部错误状态。
- [ ] store：`openMenuCount` 增减正确，`openMenuCount > 0` 时 `shouldPause` 语义为暂停。

### 6.2 组件/集成测试（Playwright 或 WebdriverIO + Tauri，`tests/player/`）

- [ ] **正常路径**：普清全屏播放，静止 3s 控制栏与指针隐藏；移动鼠标，200ms 内控制栏复现。
- [ ] **回归路径（核心）**：超清 → 普清 → 超清 来回切换 5 次，每次切换后静止 3s，控制栏均正常隐藏（FR-06 验收第 3 条）。
- [ ] **边界**：播放中展开选集下拉菜单，静止 5s 控制栏**不隐藏**；关闭菜单后再静止 3s 正常隐藏。
- [ ] **边界**：非全屏状态下静止不触发隐藏（`shouldPause` 生效）。
- [ ] **异常路径**：mock 播放地址返回 403，2 秒内出现 ErrorOverlay，显示"拒绝访问"文案与"切换源"按钮；点击"重试"2 次均失败后自动弹出源推荐列表且按健康度排序（ok 在前）。
- [ ] **异常路径**：mock 解析为空地址，显示 PARSE_EMPTY 文案，"切换源"按钮默认聚焦；点击后 `switchSource` 被调用且参数携带当前 `positionSec`（接口桩下验证调用参数）。
- [ ] **异常路径**："复制日志"后剪贴板内容包含 `error.detail` 与 `httpStatus`。
- [ ] **泄漏回归**：反复进入/退出播放页 10 次，`document` 上残留的全局 `keydown` 监听器数量为 0（通过 spy/计数验证）。

## 七、完成定义 Definition of Done

- [ ] `idleTimer` action 落地，播放器内不再存在任何其他控制栏隐藏定时器/散置 mousemove 监听。
- [ ] 画质切换复用 video 元素（或实例重建后监听正确重挂），5 次超清/普清来回切换回归测试通过。
- [ ] 控制栏隐藏/显示、指针隐藏、菜单展开暂停隐藏四个行为全部满足 FR-06 验收标准。
- [ ] ErrorOverlay 覆盖 5 类错误，403 场景 2 秒内展示且含"切换源"按钮；重试 2 次失败自动弹出按健康度排序的源推荐列表（FR-07 验收全过）。
- [ ] 源切换调用通过适配层走任务 1 接口，参数含进度保持字段；接口缺失时有明确 TODO 桩。
- [ ] 第六节全部单元测试与集成测试通过；`onDestroy` 无定时器/监听器泄漏。
- [ ] 控制栏复现延迟 < 200ms（测试断言）。
- [ ] 代码通过现有 lint / type-check，新增文件含必要注释。

## 八、禁止修改清单

- `src-tauri/` 全部（Rust 侧任何文件，含 `Cargo.toml`）。
- 规则引擎与源相关目录：`resources/rules/`、`src/lib/services/rules*`（仅允许"读取"源健康 store 接口，不得修改其实现）。
- 历史/同步相关：`src/lib/stores/history*`、任何 SQLite 迁移与 WebDAV 代码。
- CI 配置：`release.yml`、`.github/workflows/` 下其他文件（如需新增播放器测试 workflow 需单独任务批准）。
- 全局应用骨架：`src/App.svelte`、`src/main.ts`、路由入口文件。
- 播放器库依赖本身（`package.json` 中播放器相关依赖版本禁止变更；禁止 fork 依赖入库，wrapper 补丁必须落在 `src/lib/` 内）。
- 漫画/小说阅读器组件（`src/lib/components/reader/` 或等价目录）——属于任务 6 范围。
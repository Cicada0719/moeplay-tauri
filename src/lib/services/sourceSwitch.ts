import { derived, writable, type Readable } from "svelte/store";
import { uiStore } from "../stores/ui.svelte";

/**
 * 源切换适配层（FR-02 / FR-07）。
 *
 * spec §5 的「假设接口」由任务 1 / 任务 2 产出，本文件是任务 3 侧的薄适配层：
 * - `switchSource`：任务 1 的源切换服务就绪前，注册处理器为空时走 TODO 桩
 *   （弹 Toast），保证本任务可独立交付与测试；
 * - `getSourcesFor` / `sortSourcesByHealth`：任务 2 的源健康 store 就绪前，
 *   由播放器通过 `setSourceProvider` 注入真实数据源，排序逻辑先落地可单测。
 */

// ── 假设接口 1（任务 1，FR-02 源切换）：src/lib/services/sourceSwitch.ts ──
export interface SwitchSourceParams {
  contentId: string;
  /** 集/话/章标识 */
  chapterId?: string;
  /** 播放进度（秒），切换后续播 */
  positionSec?: number;
  targetSourceId: string;
}

// ── 假设接口 2（任务 2，FR-04 源健康状态）：src/lib/stores/sources.ts ──
export type SourceHealth = "ok" | "degraded" | "unknown";
export interface SourceInfo {
  id: string;
  name: string;
  contentType: string;
  health: SourceHealth;
}

type SourceSwitchHandler = (params: SwitchSourceParams) => Promise<void>;
let sourceSwitchHandler: SourceSwitchHandler | null = null;

type SourceProvider = (contentType: string) => SourceInfo[];
let sourceProvider: SourceProvider | null = null;

/** provider 注册版本号：provider 变化时 bump，供 `sourcesFor` 可读 store 响应式刷新 */
const sourceProviderVersion = writable(0);

/** 注册/注销真实源切换处理器（任务 1 服务就绪后在此接入） */
export function setSourceSwitchHandler(handler: SourceSwitchHandler | null): void {
  sourceSwitchHandler = handler;
}

/** 注册/注销源健康数据 provider（任务 2 store 就绪后可由其替代） */
export function setSourceProvider(provider: SourceProvider | null): void {
  sourceProvider = provider;
  sourceProviderVersion.update((n) => n + 1);
}

/**
 * 校验源切换参数（spec §3.5 / 外部接口健壮性）：
 * 非法参数直接抛 `TypeError`，避免脏数据流入任务 1 的源切换服务。
 * `targetSourceId` 允许空字符串——空值表示「打开选源面板」，非空时作为目标源标识。
 */
export function validateSwitchSourceParams(params: SwitchSourceParams): void {
  if (!params || typeof params !== "object") {
    throw new TypeError("switchSource 参数必须是对象");
  }
  if (typeof params.contentId !== "string" || params.contentId.trim() === "") {
    throw new TypeError("switchSource 参数 contentId 必须是非空字符串");
  }
  if (
    params.chapterId !== undefined &&
    (typeof params.chapterId !== "string" || params.chapterId.trim() === "")
  ) {
    throw new TypeError("switchSource 参数 chapterId 必须是非空字符串");
  }
  if (
    params.positionSec !== undefined &&
    (typeof params.positionSec !== "number" ||
      !Number.isFinite(params.positionSec) ||
      params.positionSec < 0)
  ) {
    throw new TypeError("switchSource 参数 positionSec 必须是非负有限数字");
  }
  if (typeof params.targetSourceId !== "string") {
    throw new TypeError("switchSource 参数 targetSourceId 必须是字符串");
  }
}

const HEALTH_RANK: Record<SourceHealth, number> = { ok: 0, unknown: 1, degraded: 2 };

/** 按健康度排序：可用 > 未知 > 异常，同级按名称 */
export function sortSourcesByHealth(sources: SourceInfo[]): SourceInfo[] {
  return [...sources].sort(
    (a, b) => HEALTH_RANK[a.health] - HEALTH_RANK[b.health] || a.name.localeCompare(b.name),
  );
}

/** 读取指定内容类型的源健康列表（未注入 provider 时返回空） */
export function getSourcesFor(contentType: string): SourceInfo[] {
  if (!sourceProvider) {
    // TODO(task-2): 任务 2 的源健康 store（src/lib/stores/sources.ts）就绪后替换。
    return [];
  }
  return sourceProvider(contentType);
}

/**
 * 源健康列表可读 store（spec §5「或对应 readable store」）：provider 重注册时自动刷新，
 * 供 SourceSuggestSheet 等组件响应式订阅，替代父组件注入的临时 sources 列表。
 */
export function sourcesFor(contentType: string): Readable<SourceInfo[]> {
  return derived(sourceProviderVersion, () => getSourcesFor(contentType));
}

/** 触发源切换：先校验参数，再优先走已注册的处理器，否则弹 TODO 桩提示 */
export async function switchSource(params: SwitchSourceParams): Promise<void> {
  validateSwitchSourceParams(params);
  if (sourceSwitchHandler) {
    return sourceSwitchHandler(params);
  }
  // TODO(task-1): FR-02 源切换服务就绪后移除桩，改为真实切换（保留进度）。
  // targetSourceId 为空字符串时表示「打开选源面板」，由播放器侧 switchSource() 流程处理。
  uiStore.toast("源切换能力待接入");
}

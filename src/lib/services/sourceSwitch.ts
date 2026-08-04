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

/** 注册/注销真实源切换处理器（任务 1 服务就绪后在此接入） */
export function setSourceSwitchHandler(handler: SourceSwitchHandler | null): void {
  sourceSwitchHandler = handler;
}

/** 注册/注销源健康数据 provider（任务 2 store 就绪后可由其替代） */
export function setSourceProvider(provider: SourceProvider | null): void {
  sourceProvider = provider;
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

/** 触发源切换：优先走已注册的处理器，否则弹 TODO 桩提示 */
export async function switchSource(params: SwitchSourceParams): Promise<void> {
  if (sourceSwitchHandler) {
    return sourceSwitchHandler(params);
  }
  // TODO(task-1): FR-02 源切换服务就绪后移除桩，改为真实切换（保留进度）。
  uiStore.toast("源切换能力待接入");
}

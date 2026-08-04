import { get, writable, type Writable } from "svelte/store";

/**
 * 播放器状态 store（FR-06 / FR-07）。
 *
 * 控制栏隐藏/显示由 `controlsVisible` 驱动，空闲计时由 `useIdleTimer` action
 * 负责；`openMenuCount` 用于在选集/画质下拉展开时暂停隐藏。播放错误由
 * `playerError` 承载，`retryPlayback` 连续失败达到阈值后置位 `showSourceSuggest`
 * 以弹出按健康度排序的源推荐列表。
 */

/** 画质档位（本地超清化：off | 均衡 | 质量） */
export type PlayerQuality = "off" | "balanced" | "quality";

export type PlayerErrorKind =
  | "NETWORK" // 网络错误/超时
  | "PARSE_EMPTY" // 解析结果为空
  | "HTTP_FORBIDDEN" // 403/防盗链
  | "HTTP_ERROR" // 其他 4xx/5xx
  | "MEDIA_DECODE"; // 解码失败

export interface PlayerError {
  kind: PlayerErrorKind;
  message: string; // 用户可读描述
  detail?: string; // 原始错误/堆栈，供复制日志
  httpStatus?: number;
  url?: string;
  occurredAt: number; // Date.now()
}

/** 控制栏可见性：true = 显示，false = 隐藏（含鼠标指针） */
export const controlsVisible: Writable<boolean> = writable(true);

/** 下拉菜单打开计数（选集/画质/评论等菜单各 +1/-1），> 0 时暂停隐藏计时 */
export const openMenuCount: Writable<number> = writable(0);

/** 播放错误状态，非空时渲染 ErrorOverlay */
export const playerError: Writable<PlayerError | null> = writable(null);

/** 当前错误已重试次数 */
export const retryCount: Writable<number> = writable(0);

/** 连续失败 ≥ 阈值后置 true，渲染 SourceSuggestSheet */
export const showSourceSuggest: Writable<boolean> = writable(false);

/** 连续重试失败达到该次数后自动弹出源推荐列表 */
export const RETRY_SUGGEST_THRESHOLD = 2;

/** 重试动作的载体：由播放器组件注册，保持 store 与 DOM/播放逻辑解耦 */
type RetryHandler = () => Promise<boolean>;
let retryHandler: RetryHandler | null = null;

/** 注册/注销重试处理器（播放器挂载/销毁时调用，避免跨实例泄漏） */
export function setRetryHandler(handler: RetryHandler | null): void {
  retryHandler = handler;
}

/** 记录错误，重置 retryCount（新的错误从 0 开始计重试） */
export function reportPlayerError(err: PlayerError): void {
  playerError.set(err);
  retryCount.set(0);
  showSourceSuggest.set(false);
}

/**
 * 重试当前播放地址。成功 → 清除错误状态；失败 → retryCount+1，
 * 连续失败达到阈值后置位 showSourceSuggest 以弹出源推荐列表。
 */
export async function retryPlayback(): Promise<void> {
  const ok = retryHandler ? await retryHandler() : true;
  if (ok) {
    clearPlayerError();
    return;
  }
  const next = get(retryCount) + 1;
  retryCount.set(next);
  if (next >= RETRY_SUGGEST_THRESHOLD) {
    showSourceSuggest.set(true);
  }
}

/** 恢复播放/切换源成功后调用，清空全部错误状态 */
export function clearPlayerError(): void {
  playerError.set(null);
  retryCount.set(0);
  showSourceSuggest.set(false);
}

export interface IdlePauseState {
  openMenuCount: number;
  isFullscreen: boolean;
  hasPlayerError: boolean;
}

/** idleTimer 的 shouldPause 语义：菜单展开 / 非全屏 / 错误弹层存在时暂停隐藏 */
export function shouldPauseIdleTimer(state: IdlePauseState): boolean {
  return state.openMenuCount > 0 || !state.isFullscreen || state.hasPlayerError;
}

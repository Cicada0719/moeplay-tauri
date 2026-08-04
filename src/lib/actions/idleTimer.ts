/**
 * `idleTimer` — 播放器控制栏空闲自动隐藏 Svelte Action（FR-06）。
 *
 * 根因（2026-08 spike）：旧实现里控制栏隐藏由 `Player` 组件内散落的
 * `setTimeout` + 组件级 `mousemove` 监听管理；切换超清画质时若重建播放器
 * 容器，旧定时器与事件监听会悬空，导致控制栏永不再隐藏。本 action 把
 * 「计时 + 事件监听 + 清理」收敛为一处，挂载在**不随画质切换重建**的稳定
 * 全屏容器上，`destroy()` 保证全部监听器与定时器被清除（无泄漏）。
 */
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

/** shouldPause 期间的轮询间隔：菜单展开时按此节奏等待条件解除 */
const POLL_INTERVAL_MS = 500;

/**
 * 挂载到播放器全屏容器，监听容器内 mousemove / mousedown / wheel / touchstart
 * 与 window 级 keydown。任意活动事件都会先退出空闲态（调用 `onActive`）再重置
 * 计时器；计时到点且 `shouldPause` 未阻塞时进入空闲态（调用 `onIdle`）。
 */
export function idleTimer(
  node: HTMLElement,
  options: IdleTimerOptions,
): { update: (opts: IdleTimerOptions) => void; destroy: () => void } {
  let { timeout = 3000, shouldPause, onIdle, onActive } = options;
  let timer: ReturnType<typeof setTimeout> | null = null;
  let pollTimer: ReturnType<typeof setTimeout> | null = null;
  let isIdle = false;
  let destroyed = false;

  const clearTimers = () => {
    if (timer !== null) {
      clearTimeout(timer);
      timer = null;
    }
    if (pollTimer !== null) {
      clearTimeout(pollTimer);
      pollTimer = null;
    }
  };

  const enterIdle = () => {
    clearTimers();
    if (destroyed) return;
    isIdle = true;
    onIdle();
  };

  /** 计时到点：若被 `shouldPause` 阻塞则转入 500ms 轮询，否则进入空闲态 */
  const checkIdle = () => {
    if (destroyed) return;
    timer = null;
    if (shouldPause?.()) {
      pollTimer = setTimeout(resumeCountdown, POLL_INTERVAL_MS);
      return;
    }
    enterIdle();
  };

  /** 轮询等待 `shouldPause` 解除；解除后重新走一轮完整计时（菜单刚关闭不立刻隐藏） */
  const resumeCountdown = () => {
    if (destroyed) return;
    pollTimer = null;
    if (shouldPause?.()) {
      pollTimer = setTimeout(resumeCountdown, POLL_INTERVAL_MS);
      return;
    }
    resetTimer();
  };

  const resetTimer = () => {
    clearTimers();
    if (destroyed) return;
    if (isIdle) {
      isIdle = false;
      onActive();
    }
    timer = setTimeout(checkIdle, timeout);
  };

  const onActivity = () => resetTimer();

  node.addEventListener("mousemove", onActivity);
  node.addEventListener("mousedown", onActivity);
  node.addEventListener("wheel", onActivity);
  node.addEventListener("touchstart", onActivity);
  window.addEventListener("keydown", onActivity);

  resetTimer();

  return {
    update(next) {
      timeout = next.timeout ?? timeout;
      shouldPause = next.shouldPause;
      onIdle = next.onIdle;
      onActive = next.onActive;
      resetTimer();
    },
    destroy() {
      destroyed = true;
      clearTimers();
      node.removeEventListener("mousemove", onActivity);
      node.removeEventListener("mousedown", onActivity);
      node.removeEventListener("wheel", onActivity);
      node.removeEventListener("touchstart", onActivity);
      window.removeEventListener("keydown", onActivity);
    },
  };
}

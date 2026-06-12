// Switch/PS5 外壳共用的手柄导航 composable。
// 用 Gamepad API + requestAnimationFrame 轮询，按钮做边沿去抖（按下→释放才触发一次）。
// 仅在有手柄连接且窗口聚焦时轮询；返回 detach 清理函数。
//
// 按键映射（Xbox / PS 通用下标）：
//   0 = A / ✕  → launch（启动）
//   1 = B / ○  → back
//   2 = X / □  → favorite
//   3 = Y / △  → activate（详情）
//   4/5 = LB/RB → pageLeft/pageRight
//   12-15 = 十字键 上/下/左/右
// 左摇杆水平轴用 latch 去抖。

export type GamepadHandlers = {
  left?: () => void;
  right?: () => void;
  pageLeft?: () => void;
  pageRight?: () => void;
  activate?: () => void;
  launch?: () => void;
  favorite?: () => void;
  back?: () => void;
};

const BTN = { A: 0, B: 1, X: 2, Y: 3, LB: 4, RB: 5, DUP: 12, DDOWN: 13, DLEFT: 14, DRIGHT: 15 };

export function attachGamepad(handlers: GamepadHandlers): () => void {
  if (typeof navigator === "undefined" || typeof navigator.getGamepads !== "function") {
    return () => {};
  }

  let raf = 0;
  let running = false;
  const prev: Record<number, boolean> = {};
  let axisLatch = 0; // -1 | 0 | 1

  const edge = (i: number, pressed: boolean): boolean => {
    const was = prev[i] ?? false;
    prev[i] = pressed;
    return pressed && !was;
  };

  const anyPad = (): boolean =>
    Array.from(navigator.getGamepads?.() ?? []).some((p) => p != null);

  const loop = () => {
    if (!running) return;
    raf = requestAnimationFrame(loop);

    if (typeof document !== "undefined" && typeof document.hasFocus === "function" && !document.hasFocus()) {
      return;
    }

    const pad = Array.from(navigator.getGamepads?.() ?? []).find((p) => p != null);
    if (!pad) return;

    const b = pad.buttons;
    const ax = pad.axes[0] ?? 0;

    if (ax < -0.5) {
      if (axisLatch !== -1) { axisLatch = -1; handlers.left?.(); }
    } else if (ax > 0.5) {
      if (axisLatch !== 1) { axisLatch = 1; handlers.right?.(); }
    } else {
      axisLatch = 0;
    }

    if (edge(BTN.DLEFT, !!b[BTN.DLEFT]?.pressed)) handlers.left?.();
    if (edge(BTN.DRIGHT, !!b[BTN.DRIGHT]?.pressed)) handlers.right?.();
    if (edge(BTN.LB, !!b[BTN.LB]?.pressed)) (handlers.pageLeft ?? handlers.left)?.();
    if (edge(BTN.RB, !!b[BTN.RB]?.pressed)) (handlers.pageRight ?? handlers.right)?.();
    if (edge(BTN.A, !!b[BTN.A]?.pressed)) handlers.launch?.();
    if (edge(BTN.Y, !!b[BTN.Y]?.pressed)) handlers.activate?.();
    if (edge(BTN.X, !!b[BTN.X]?.pressed)) handlers.favorite?.();
    if (edge(BTN.B, !!b[BTN.B]?.pressed)) handlers.back?.();
  };

  const start = () => {
    if (running) return;
    running = true;
    loop();
  };
  const stop = () => {
    running = false;
    cancelAnimationFrame(raf);
  };

  const onConnect = () => start();
  const onDisconnect = () => { if (!anyPad()) stop(); };

  window.addEventListener("gamepadconnected", onConnect);
  window.addEventListener("gamepaddisconnected", onDisconnect);
  if (anyPad()) start();

  return () => {
    stop();
    window.removeEventListener("gamepadconnected", onConnect);
    window.removeEventListener("gamepaddisconnected", onDisconnect);
  };
}

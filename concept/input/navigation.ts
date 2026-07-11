import type { ContentMode, NavigationIntent } from "../contracts";

export interface NavigationControllerOptions {
  getMode: () => ContentMode;
  onIntent: (intent: NavigationIntent, event: Event) => void;
  disabled?: boolean;
  sceneThreshold?: number;
  sceneCooldownMs?: number;
  touchThreshold?: number;
}

const EDITABLE_SELECTOR = "input, textarea, select, [contenteditable='true'], [role='textbox']";

function isEditable(target: EventTarget | null): boolean {
  return target instanceof Element && Boolean(target.closest(EDITABLE_SELECTOR));
}

function keyboardIntent(event: KeyboardEvent): NavigationIntent | null {
  if ((event.altKey || event.shiftKey) && event.key === "ArrowLeft") return "switch-mode-left";
  if ((event.altKey || event.shiftKey) && event.key === "ArrowRight") return "switch-mode-right";
  switch (event.key) {
    case "ArrowDown": return "next";
    case "ArrowUp": return "previous";
    case "ArrowRight": return "next";
    case "ArrowLeft": return "previous";
    case "PageDown": return "page-next";
    case "PageUp": return "page-previous";
    case "Enter":
    case " ": return "activate";
    case "Escape": return "back";
    default: return null;
  }
}

export function navigationController(node: HTMLElement, options: NavigationControllerOptions) {
  let config = options;
  let sceneDelta = 0;
  let sceneLockedUntil = 0;
  let touchStart: { x: number; y: number } | null = null;
  let gamepadFrame = 0;
  let gamepadCooldownUntil = 0;
  const pressedButtons = new Set<number>();

  const emit = (intent: NavigationIntent, event: Event) => config.onIntent(intent, event);

  const onKeyDown = (event: KeyboardEvent) => {
    if (config.disabled || isEditable(event.target) || event.defaultPrevented) return;
    const intent = keyboardIntent(event);
    if (!intent) return;
    event.preventDefault();
    emit(intent, event);
  };

  const onWheel = (event: WheelEvent) => {
    if (config.disabled || event.defaultPrevented) return;
    const mode = config.getMode();
    if (mode === "index") return;

    const rail = event.target instanceof Element ? event.target.closest<HTMLElement>("[data-concept-wheel='intent']") : null;
    if (mode === "visual" && !rail) return;

    if (mode === "visual") {
      const horizontal = rail?.dataset.conceptAxis === "horizontal";
      const delta = horizontal ? (Math.abs(event.deltaX) > Math.abs(event.deltaY) ? event.deltaX : event.deltaY) : event.deltaY;
      if (Math.abs(delta) < 2) return;
      event.preventDefault();
      emit(delta > 0 ? "next" : "previous", event);
      return;
    }

    event.preventDefault();
    const now = performance.now();
    if (now < sceneLockedUntil) return;
    if (sceneDelta !== 0 && Math.sign(sceneDelta) !== Math.sign(event.deltaY)) sceneDelta = 0;
    sceneDelta += event.deltaY;
    const threshold = config.sceneThreshold ?? 72;
    if (Math.abs(sceneDelta) < threshold) return;
    emit(sceneDelta > 0 ? "next" : "previous", event);
    sceneDelta = 0;
    sceneLockedUntil = now + (config.sceneCooldownMs ?? 520);
  };

  const onTouchStart = (event: TouchEvent) => {
    if (config.disabled || event.touches.length !== 1) return;
    touchStart = { x: event.touches[0].clientX, y: event.touches[0].clientY };
  };

  const onTouchEnd = (event: TouchEvent) => {
    if (config.disabled || !touchStart || event.changedTouches.length !== 1) return;
    const end = event.changedTouches[0];
    const dx = end.clientX - touchStart.x;
    const dy = end.clientY - touchStart.y;
    touchStart = null;
    const threshold = config.touchThreshold ?? 48;
    if (Math.max(Math.abs(dx), Math.abs(dy)) < threshold) return;
    const intent = Math.abs(dx) > Math.abs(dy)
      ? (dx < 0 ? "next" : "previous")
      : (dy < 0 ? "next" : "previous");
    emit(intent, event);
  };


  const pollGamepads = (time: number) => {
    gamepadFrame = requestAnimationFrame(pollGamepads);
    if (config.disabled || typeof navigator.getGamepads !== "function") return;
    const gamepad = Array.from(navigator.getGamepads()).find(Boolean);
    if (!gamepad) return;

    const buttonMap: Array<[number, NavigationIntent]> = [
      [12, "previous"], [13, "next"], [14, "previous"], [15, "next"],
      [0, "activate"], [1, "back"], [4, "switch-mode-left"], [5, "switch-mode-right"],
    ];
    for (const [index, intent] of buttonMap) {
      const down = gamepad.buttons[index]?.pressed === true;
      if (down && !pressedButtons.has(index)) emit(intent, new Event("gamepad"));
      if (down) pressedButtons.add(index); else pressedButtons.delete(index);
    }

    const axis = Math.abs(gamepad.axes[1] ?? 0) >= Math.abs(gamepad.axes[0] ?? 0)
      ? gamepad.axes[1] ?? 0
      : gamepad.axes[0] ?? 0;
    if (Math.abs(axis) > 0.72 && time >= gamepadCooldownUntil) {
      emit(axis > 0 ? "next" : "previous", new Event("gamepad"));
      gamepadCooldownUntil = time + 330;
    }
  };
  node.addEventListener("keydown", onKeyDown);
  node.addEventListener("wheel", onWheel, { passive: false });
  node.addEventListener("touchstart", onTouchStart, { passive: true });
  node.addEventListener("touchend", onTouchEnd, { passive: true });
  gamepadFrame = requestAnimationFrame(pollGamepads);

  return {
    update(next: NavigationControllerOptions) { config = next; },
    destroy() {
      if (gamepadFrame) cancelAnimationFrame(gamepadFrame);
      gamepadFrame = 0;
      pressedButtons.clear();
      node.removeEventListener("keydown", onKeyDown);
      node.removeEventListener("wheel", onWheel);
      node.removeEventListener("touchstart", onTouchStart);
      node.removeEventListener("touchend", onTouchEnd);
    },
  };
}


import {
  getGamepadLayoutRevision,
  mapFaceButton,
  readGamepadLayoutPreference,
  resolveGamepadLayout,
  type GamepadLayout,
} from "../../platform/gamepadLayout";
import {
  ACTION_BY_SEMANTIC,
  getGamepadRemapRevision,
  readGamepadRemap,
  type GamepadRemap,
} from "../../platform/gamepadRemap";
import { gamepadTuning, getGamepadTuningRevision } from "../../platform/gamepadTuning.svelte";

export type GamepadDirection = "up" | "down" | "left" | "right";
export type GamepadInputMode = "gamepad" | "keyboard";
export type GamepadZone = string;

export interface GamepadButtonLike {
  pressed: boolean;
  value?: number;
}

export interface GamepadLike {
  connected?: boolean;
  buttons: ArrayLike<GamepadButtonLike>;
  axes: ArrayLike<number>;
  /** 手柄 id（如 "Nintendo Switch Pro Controller"），用于按键布局识别 */
  readonly id?: string;
  readonly mapping?: string;
  /** 手柄槽位序号（多手柄合并输入时区分滞回状态） */
  readonly index?: number;
}

export interface GamepadNavigatorLike {
  getGamepads(): ArrayLike<GamepadLike | null>;
}

export interface GamepadClock {
  now(): number;
  requestFrame(callback: (timestamp: number) => void): number;
  cancelFrame(handle: number): void;
}

export interface GamepadRuntimeEnvironment {
  navigator: GamepadNavigatorLike;
  clock: GamepadClock;
  connectionEvents?: Pick<EventTarget, "addEventListener" | "removeEventListener">;
  keyboardEvents?: Pick<EventTarget, "addEventListener" | "removeEventListener">;
  hasFocus?: () => boolean;
}

export interface GamepadScopeHandlers {
  up?: () => void;
  down?: () => void;
  left?: () => void;
  right?: () => void;
  pageLeft?: () => void;
  pageRight?: () => void;
  /** LT / L2：切换上一级频道或自定义分类。 */
  categoryLeft?: () => void;
  /** RT / R2：切换下一级频道或自定义分类。 */
  categoryRight?: () => void;
  activate?: () => void;
  launch?: () => void;
  favorite?: () => void;
  filter?: () => void;
  back?: () => void;
  start?: () => void;
}

export interface GamepadScopeOptions {
  id?: string;
  priority?: number;
  overlay?: boolean;
  zone?: GamepadZone | null;
  enabled?: boolean;
  paused?: boolean;
}

export interface GamepadScopeController {
  readonly id: string;
  readonly paused: boolean;
  readonly destroyed: boolean;
  pause(): void;
  resume(): void;
  activate(): void;
  setZone(zone: GamepadZone | null): void;
  setPriority(priority: number): void;
  setOverlay(overlay: boolean): void;
  setEnabled(enabled: boolean): void;
  updateHandlers(handlers: GamepadScopeHandlers): void;
  destroy(): void;
}

export interface GamepadRuntimeOptions {
  initialRepeatDelayMs?: number;
  repeatIntervalMs?: number;
  axisPressThreshold?: number;
  axisReleaseThreshold?: number;
}

type DirectionState = {
  held: boolean;
  nextAt: number;
};

type InputSample = {
  directions: Record<GamepadDirection, boolean>;
  buttons: Map<number, boolean>;
};

type ScopeEntry = {
  id: string;
  handlers: GamepadScopeHandlers;
  priority: number;
  overlay: boolean;
  zone: GamepadZone | null;
  enabled: boolean;
  paused: boolean;
  order: number;
};

const BUTTON = {
  A: 0,
  B: 1,
  X: 2,
  Y: 3,
  LB: 4,
  RB: 5,
  LT: 6,
  RT: 7,
  VIEW: 8,
  START: 9,
  DPAD_UP: 12,
  DPAD_DOWN: 13,
  DPAD_LEFT: 14,
  DPAD_RIGHT: 15,
} as const;

const DIRECTIONS: readonly GamepadDirection[] = ["up", "down", "left", "right"];
const EDGE_BUTTONS = [BUTTON.A, BUTTON.B, BUTTON.X, BUTTON.Y, BUTTON.LB, BUTTON.RB, BUTTON.LT, BUTTON.RT, BUTTON.VIEW, BUTTON.START] as const;
const ANALOG_BUTTONS = new Set<number>([BUTTON.LT, BUTTON.RT]);
const MODIFIER_KEYS = new Set(["Alt", "AltGraph", "Control", "Meta", "Shift", "CapsLock", "NumLock", "ScrollLock"]);

function makeDirectionState(): Record<GamepadDirection, DirectionState> {
  return {
    up: { held: false, nextAt: 0 },
    down: { held: false, nextAt: 0 },
    left: { held: false, nextAt: 0 },
    right: { held: false, nextAt: 0 },
  };
}

function safePressed(buttons: ArrayLike<GamepadButtonLike>, index: number): boolean {
  return Boolean(buttons[index]?.pressed || (buttons[index]?.value ?? 0) >= 0.5);
}

/** AIR X 的 Android 映射会让扳机静止时仍报告 pressed=true，必须使用相对基线。 */
function analogPressed(value: number, baseline: number, held: boolean): boolean {
  const press = Math.min(0.95, Math.max(0.65, baseline + 0.16));
  const release = Math.min(0.85, Math.max(0.45, baseline + 0.08));
  return held ? value >= release : value >= press;
}

function defaultEnvironment(): GamepadRuntimeEnvironment | null {
  if (typeof navigator === "undefined" || typeof navigator.getGamepads !== "function") return null;
  if (typeof requestAnimationFrame !== "function" || typeof cancelAnimationFrame !== "function") return null;

  const eventTarget = typeof window !== "undefined" ? window : undefined;
  return {
    navigator: navigator as unknown as GamepadNavigatorLike,
    clock: {
      now: () => (typeof performance !== "undefined" ? performance.now() : Date.now()),
      requestFrame: (callback) => requestAnimationFrame(callback),
      cancelFrame: (handle) => cancelAnimationFrame(handle),
    },
    connectionEvents: eventTarget,
    keyboardEvents: eventTarget,
    hasFocus: () => typeof document === "undefined" || typeof document.hasFocus !== "function" || document.hasFocus(),
  };
}

/**
 * Shared gamepad input runtime.
 *
 * Exactly one eligible scope receives input at a time. A top-most overlay wins
 * first; otherwise the highest-priority scope in the active zone wins, with
 * registration/activation order acting as the stack tie-breaker.
 */
export class GamepadFocusRuntime {
  private readonly scopes: ScopeEntry[] = [];
  private readonly directionState = makeDirectionState();
  private readonly buttonState = new Map<number, boolean>();
  private readonly modeListeners = new Set<(mode: GamepadInputMode) => void>();
  private readonly initialRepeatDelayMs: number;
  private readonly repeatIntervalMs: number;
  private readonly axisPressThreshold: number;
  private readonly axisReleaseThreshold: number;
  private readonly useTuning: boolean;
  private activeZone: GamepadZone | null = null;
  private faceLayouts = new Map<string, GamepadLayout>();
  private faceLayoutsRevision = getGamepadLayoutRevision();
  private remapCache: GamepadRemap | null = null;
  private remapRevision = getGamepadRemapRevision();
  private tuningRevision = getGamepadTuningRevision();
  private tuning = {
    press: gamepadTuning.axisPress,
    release: gamepadTuning.axisRelease,
    interval: gamepadTuning.repeatIntervalMs,
    initial: gamepadTuning.initialDelayMs,
  };
  private activeScopeId: string | null = null;
  private inputMode: GamepadInputMode = "keyboard";
  private frameHandle: number | null = null;
  private running = false;
  private globallyPaused = false;
  private listenersInstalled = false;
  private awaitingNeutralAfterKeyboard = false;
  private awaitingNeutralAfterScopeChange = false;
  private axisStates = new Map<number, { h: -1 | 0 | 1; v: -1 | 0 | 1 }>();
  private triggerBaselines = new Map<string, number>();
  private triggerStates = new Map<string, boolean>();
  private order = 0;
  private sequence = 0;

  private readonly onConnected = () => this.ensureLoop();
  private readonly onDisconnected = () => {
    if (!this.findPad()) {
      this.stopLoop();
      this.resetInputState();
    }
  };
  private readonly onKeyboardInput = (event: Event) => {
    // Only genuine user key presses may steal input mode. Programmatic key
    // events (e.g. the controller-surface bridge forwarding stick directions
    // as arrow keys) are untrusted and must not flip the mode back.
    if (!event.isTrusted) return;
    const key = "key" in event && typeof event.key === "string" ? event.key : "";
    if (MODIFIER_KEYS.has(key)) return;
    this.takeOverWithKeyboard();
  };

  constructor(
    private readonly environment: GamepadRuntimeEnvironment,
    options: GamepadRuntimeOptions = {},
  ) {
    this.initialRepeatDelayMs = options.initialRepeatDelayMs ?? 320;
    this.repeatIntervalMs = options.repeatIntervalMs ?? 100;
    this.axisPressThreshold = options.axisPressThreshold ?? 0.55;
    this.axisReleaseThreshold = options.axisReleaseThreshold ?? 0.35;
    // 未显式传参（默认构造/测试常用参数化构造除外）时跟随设置页灵敏度调参
    this.useTuning = options.initialRepeatDelayMs === undefined
      && options.repeatIntervalMs === undefined
      && options.axisPressThreshold === undefined
      && options.axisReleaseThreshold === undefined;

    if (this.initialRepeatDelayMs < 0 || this.repeatIntervalMs <= 0) {
      throw new Error("Gamepad repeat timing must be non-negative with a positive interval");
    }
    if (this.axisReleaseThreshold < 0 || this.axisPressThreshold <= this.axisReleaseThreshold) {
      throw new Error("Gamepad axis thresholds require press > release >= 0");
    }
  }

  registerScope(
    handlers: GamepadScopeHandlers,
    options: GamepadScopeOptions = {},
  ): GamepadScopeController {
    const entry: ScopeEntry = {
      id: options.id ?? `gamepad-scope-${++this.sequence}`,
      handlers,
      priority: options.priority ?? 0,
      overlay: options.overlay ?? false,
      zone: options.zone ?? null,
      enabled: options.enabled ?? true,
      paused: options.paused ?? false,
      order: ++this.order,
    };

    if (this.scopes.some((scope) => scope.id === entry.id)) {
      throw new Error(`Gamepad scope id already registered: ${entry.id}`);
    }

    this.scopes.push(entry);
    this.installListeners();
    this.onScopeTopologyChanged();
    this.ensureLoop();

    let destroyed = false;
    const mutate = (callback: () => void) => {
      if (destroyed) return;
      callback();
      this.onScopeTopologyChanged();
      this.ensureLoop();
    };

    return {
      get id() { return entry.id; },
      get paused() { return entry.paused; },
      get destroyed() { return destroyed; },
      pause: () => mutate(() => { entry.paused = true; }),
      resume: () => {
        if (destroyed) return;
        entry.paused = false;
        entry.order = ++this.order;
        this.onScopeTopologyChanged();
        this.awaitingNeutralAfterScopeChange = true;
        this.ensureLoop();
      },
      activate: () => mutate(() => { entry.order = ++this.order; }),
      setZone: (zone) => mutate(() => { entry.zone = zone; }),
      setPriority: (priority) => mutate(() => { entry.priority = priority; }),
      setOverlay: (overlay) => mutate(() => { entry.overlay = overlay; }),
      setEnabled: (enabled) => mutate(() => { entry.enabled = enabled; }),
      updateHandlers: (nextHandlers) => mutate(() => { entry.handlers = nextHandlers; }),
      destroy: () => {
        if (destroyed) return;
        destroyed = true;
        const index = this.scopes.indexOf(entry);
        if (index >= 0) this.scopes.splice(index, 1);
        this.onScopeTopologyChanged();
        if (this.scopes.length === 0) {
          this.stopLoop();
          this.removeListeners();
        } else {
          this.ensureLoop();
        }
      },
    };
  }

  setActiveZone(zone: GamepadZone | null): void {
    if (this.activeZone === zone) return;
    this.activeZone = zone;
    this.onScopeTopologyChanged();
  }

  getActiveZone(): GamepadZone | null {
    return this.activeZone;
  }

  getActiveScopeId(): string | null {
    return this.selectActiveScope()?.id ?? null;
  }

  getInputMode(): GamepadInputMode {
    return this.inputMode;
  }

  /** 当前已连接手柄的 id / 槽位 / 最终布局（供提示条与设置页展示、诊断识别结果） */
  getConnectedPads(): { id: string; index: number; layout: GamepadLayout }[] {
    return this.findPads().map((pad) => ({
      id: pad.id ?? "",
      index: typeof pad.index === "number" ? pad.index : 0,
      layout: this.faceLayoutFor(pad),
    }));
  }

  subscribeInputMode(listener: (mode: GamepadInputMode) => void): () => void {
    this.modeListeners.add(listener);
    listener(this.inputMode);
    return () => this.modeListeners.delete(listener);
  }

  pause(): void {
    if (this.globallyPaused) return;
    this.globallyPaused = true;
    this.resetInputState();
    this.stopLoop();
  }

  resume(): void {
    if (!this.globallyPaused) return;
    this.globallyPaused = false;
    this.onScopeTopologyChanged();
    this.awaitingNeutralAfterScopeChange = true;
    this.ensureLoop();
  }

  isPaused(): boolean {
    return this.globallyPaused;
  }

  takeOverWithKeyboard(): void {
    this.setInputMode("keyboard");
    const pads = this.findPads();
    this.awaitingNeutralAfterKeyboard = pads.length > 0 ? !this.sampleIsNeutral(this.readSample(pads)) : false;
    this.resetInputState();
  }

  /** Poll once. Public for deterministic tests and non-RAF hosts. */
  poll(now = this.environment.clock.now()): void {
    if (this.globallyPaused || this.scopes.length === 0) return;
    if (this.environment.hasFocus && !this.environment.hasFocus()) {
      this.resetInputState();
      return;
    }

    const pads = this.findPads();
    if (pads.length === 0) {
      this.resetInputState();
      return;
    }

    const sample = this.readSample(pads);
    const scope = this.selectActiveScope();
    if (!scope) {
      this.syncInputState(sample, now);
      this.activeScopeId = null;
      return;
    }

    if (scope.id !== this.activeScopeId) {
      this.activeScopeId = scope.id;
      this.syncInputState(sample, now);
      return;
    }

    if (this.awaitingNeutralAfterScopeChange) {
      if (!this.sampleIsNeutral(sample)) {
        this.syncInputState(sample, now);
        return;
      }
      this.awaitingNeutralAfterScopeChange = false;
      this.resetInputState();
      return;
    }

    if (this.awaitingNeutralAfterKeyboard) {
      if (!this.sampleIsNeutral(sample)) {
        this.syncInputState(sample, now);
        return;
      }
      this.awaitingNeutralAfterKeyboard = false;
      this.resetInputState();
      return;
    }

    const directionInput = this.dispatchDirections(scope, sample.directions, now);
    const buttonInput = this.dispatchButtons(scope, sample.buttons);
    if (directionInput || buttonInput) this.setInputMode("gamepad");
  }

  destroy(): void {
    this.scopes.splice(0);
    this.stopLoop();
    this.removeListeners();
    this.modeListeners.clear();
    this.activeScopeId = null;
    this.resetInputState();
  }

  private installListeners(): void {
    if (this.listenersInstalled) return;
    this.environment.connectionEvents?.addEventListener("gamepadconnected", this.onConnected);
    this.environment.connectionEvents?.addEventListener("gamepaddisconnected", this.onDisconnected);
    this.environment.keyboardEvents?.addEventListener("keydown", this.onKeyboardInput);
    this.listenersInstalled = true;
  }

  private removeListeners(): void {
    if (!this.listenersInstalled) return;
    this.environment.connectionEvents?.removeEventListener("gamepadconnected", this.onConnected);
    this.environment.connectionEvents?.removeEventListener("gamepaddisconnected", this.onDisconnected);
    this.environment.keyboardEvents?.removeEventListener("keydown", this.onKeyboardInput);
    this.listenersInstalled = false;
  }

  private ensureLoop(): void {
    if (this.running || this.globallyPaused || this.scopes.length === 0 || !this.findPad()) return;
    this.running = true;
    this.frameHandle = this.environment.clock.requestFrame(this.runFrame);
  }

  private readonly runFrame = (timestamp: number) => {
    if (!this.running) return;
    this.frameHandle = null;
    this.poll(timestamp);

    if (!this.globallyPaused && this.scopes.length > 0 && this.findPad()) {
      this.frameHandle = this.environment.clock.requestFrame(this.runFrame);
    } else {
      this.running = false;
    }
  };

  private stopLoop(): void {
    if (this.frameHandle != null) this.environment.clock.cancelFrame(this.frameHandle);
    this.frameHandle = null;
    this.running = false;
  }

  /** 语义面键索引 → 物理按钮索引：显式重绑优先，否则按布局默认换位（带 revision 缓存） */
  private physicalFor(semanticIndex: number, layout: GamepadLayout): number {
    const revision = getGamepadRemapRevision();
    // 缓存为空（runtime 在写入之后才构造）或版本变化时重载
    if (revision !== this.remapRevision || this.remapCache === null) {
      this.remapCache = readGamepadRemap();
      this.remapRevision = revision;
    }
    const action = ACTION_BY_SEMANTIC[semanticIndex];
    if (!action) return semanticIndex;
    const explicit = this.remapCache?.[action];
    if (explicit !== undefined) return explicit;
    return mapFaceButton(layout, semanticIndex);
  }

  /** 灵敏度调参缓存：设置变更（revision 递增）后刷新 */
  private syncTuning(): void {
    const revision = getGamepadTuningRevision();
    if (revision === this.tuningRevision) return;
    this.tuningRevision = revision;
    this.tuning = {
      press: gamepadTuning.axisPress,
      release: gamepadTuning.axisRelease,
      interval: gamepadTuning.repeatIntervalMs,
      initial: gamepadTuning.initialDelayMs,
    };
  }

  private effectiveInitialDelayMs(): number {
    if (!this.useTuning) return this.initialRepeatDelayMs;
    this.syncTuning();
    return this.tuning.initial;
  }

  private effectiveRepeatIntervalMs(): number {
    if (!this.useTuning) return this.repeatIntervalMs;
    this.syncTuning();
    return this.tuning.interval;
  }

  private effectiveAxisThresholds(): { press: number; release: number } {
    if (!this.useTuning) return { press: this.axisPressThreshold, release: this.axisReleaseThreshold };
    this.syncTuning();
    return { press: this.tuning.press, release: this.tuning.release };
  }

  /** 按手柄 id + 槽位 + 用户偏好解析该手柄的面键布局（逐手柄缓存，换柄/改设置自动重算） */
  private faceLayoutFor(pad: GamepadLike): GamepadLayout {
    const override = readGamepadLayoutPreference();
    const revision = getGamepadLayoutRevision();
    if (revision !== this.faceLayoutsRevision) {
      this.faceLayouts.clear();
      this.faceLayoutsRevision = revision;
    }
    const index = typeof pad.index === "number" ? pad.index : undefined;
    const key = (pad.id ?? "") + "|" + (index ?? -1) + "|" + override;
    const cached = this.faceLayouts.get(key);
    if (cached) return cached;
    const layout = resolveGamepadLayout(pad.id ?? "", override, index);
    this.faceLayouts.set(key, layout);
    return layout;
  }

  /** 所有已连接手柄（串流/映射工具常注册多个虚拟手柄，真实手柄可能不在首位） */
  private findPads(): GamepadLike[] {
    try {
      return Array.from(this.environment.navigator.getGamepads() ?? [])
        .filter((pad): pad is GamepadLike => pad != null && pad.connected !== false);
    } catch {
      return [];
    }
  }

  private findPad(): GamepadLike | null {
    return this.findPads()[0] ?? null;
  }

  private selectActiveScope(): ScopeEntry | null {
    const eligible = this.scopes.filter((scope) => scope.enabled && !scope.paused);
    if (eligible.length === 0) return null;

    const overlays = eligible.filter((scope) => scope.overlay);
    if (overlays.length > 0) return this.highestRanked(overlays);

    if (this.activeZone != null) {
      const zoneMatches = eligible.filter((scope) => scope.zone === this.activeZone);
      if (zoneMatches.length > 0) return this.highestRanked(zoneMatches);
    }

    const globalScopes = eligible.filter((scope) => scope.zone == null);
    return this.highestRanked(globalScopes.length > 0 ? globalScopes : eligible);
  }

  private highestRanked(scopes: ScopeEntry[]): ScopeEntry {
    return scopes.reduce((winner, scope) => {
      if (scope.priority !== winner.priority) return scope.priority > winner.priority ? scope : winner;
      return scope.order > winner.order ? scope : winner;
    });
  }

  private onScopeTopologyChanged(): void {
    const nextId = this.selectActiveScope()?.id ?? null;
    if (nextId !== this.activeScopeId) {
      const previousId = this.activeScopeId;
      this.activeScopeId = nextId;
      this.awaitingNeutralAfterScopeChange = previousId != null && nextId != null;
      this.resetInputState();
    }
  }

  private readAxis(value: number, current: -1 | 0 | 1): -1 | 0 | 1 {
    const { press, release } = this.effectiveAxisThresholds();
    if (current === -1 && value <= -release) return -1;
    if (current === 1 && value >= release) return 1;
    if (value <= -press) return -1;
    if (value >= press) return 1;
    return 0;
  }

  private readSample(pads: GamepadLike[]): InputSample {
    let left = false;
    let right = false;
    let up = false;
    let down = false;
    const buttons = new Map<number, boolean>(EDGE_BUTTONS.map((index) => [index, false]));

    for (const pad of pads) {
      // 方向：十字键 OR 左摇杆（逐槽位滞回状态）
      const axisKey = typeof pad.index === "number" ? pad.index : 0;
      const prev = this.axisStates.get(axisKey) ?? { h: 0 as const, v: 0 as const };
      const h = this.readAxis(Number(pad.axes[0] ?? 0), prev.h);
      const v = this.readAxis(Number(pad.axes[1] ?? 0), prev.v);
      this.axisStates.set(axisKey, { h, v });

      left ||= safePressed(pad.buttons, BUTTON.DPAD_LEFT) || h === -1;
      right ||= safePressed(pad.buttons, BUTTON.DPAD_RIGHT) || h === 1;
      up ||= safePressed(pad.buttons, BUTTON.DPAD_UP) || v === -1;
      down ||= safePressed(pad.buttons, BUTTON.DPAD_DOWN) || v === 1;

      // 面键按该手柄的布局（Xbox/任天堂）+ 按键绑定（重映射）取物理索引后合并
      const layout = this.faceLayoutFor(pad);
      for (const index of EDGE_BUTTONS) {
        const physical = this.physicalFor(index, layout);
        if (ANALOG_BUTTONS.has(physical)) {
          const value = Number(pad.buttons[physical]?.value ?? 0);
          const key = `${pad.id ?? ""}|${pad.index ?? 0}|${physical}`;
          const previousBaseline = this.triggerBaselines.get(key);
          // 低于历史基线代表释放区间，逐渐吸收硬件的实际静止值。
          const baseline = previousBaseline === undefined ? value : Math.min(previousBaseline, value);
          this.triggerBaselines.set(key, baseline);
          const held = this.triggerStates.get(key) ?? false;
          const pressed = analogPressed(value, baseline, held);
          this.triggerStates.set(key, pressed);
          if (pressed) buttons.set(index, true);
        } else if (safePressed(pad.buttons, physical)) {
          buttons.set(index, true);
        }
      }
    }

    if (left && right) left = right = false;
    if (up && down) up = down = false;

    return {
      directions: { up, down, left, right } satisfies Record<GamepadDirection, boolean>,
      buttons,
    };
  }

  private sampleIsNeutral(sample: InputSample): boolean {
    return DIRECTIONS.every((direction) => !sample.directions[direction])
      && EDGE_BUTTONS.every((button) => !sample.buttons.get(button));
  }

  private syncInputState(sample: InputSample, now: number): void {
    for (const direction of DIRECTIONS) {
      const held = sample.directions[direction];
      this.directionState[direction] = {
        held,
        nextAt: held ? now + this.effectiveInitialDelayMs() : 0,
      };
    }
    for (const button of EDGE_BUTTONS) this.buttonState.set(button, Boolean(sample.buttons.get(button)));
  }

  private dispatchDirections(
    scope: ScopeEntry,
    directions: Record<GamepadDirection, boolean>,
    now: number,
  ): boolean {
    let dispatched = false;
    for (const direction of DIRECTIONS) {
      const state = this.directionState[direction];
      const pressed = directions[direction];
      if (!pressed) {
        state.held = false;
        state.nextAt = 0;
        continue;
      }

      if (!state.held) {
        state.held = true;
        state.nextAt = now + this.effectiveInitialDelayMs();
        scope.handlers[direction]?.();
        dispatched = true;
      } else if (now >= state.nextAt) {
        state.nextAt += this.effectiveRepeatIntervalMs();
        scope.handlers[direction]?.();
        dispatched = true;
      }
    }
    return dispatched;
  }

  private dispatchButtons(scope: ScopeEntry, buttons: Map<number, boolean>): boolean {
    let dispatched = false;
    const edge = (index: number, handler: (() => void) | undefined) => {
      const pressed = Boolean(buttons.get(index));
      const wasPressed = this.buttonState.get(index) ?? false;
      this.buttonState.set(index, pressed);
      if (pressed && !wasPressed && handler) {
        handler();
        dispatched = true;
      }
    };

    edge(BUTTON.LB, scope.handlers.pageLeft);
    edge(BUTTON.RB, scope.handlers.pageRight);
    edge(BUTTON.LT, scope.handlers.categoryLeft);
    edge(BUTTON.RT, scope.handlers.categoryRight);
    edge(BUTTON.A, scope.handlers.launch);
    edge(BUTTON.Y, scope.handlers.activate);
    edge(BUTTON.X, scope.handlers.favorite);
    edge(BUTTON.VIEW, scope.handlers.filter);
    edge(BUTTON.B, scope.handlers.back);
    const startHandler = scope.handlers.start ?? (scope.overlay ? undefined : this.findFallbackHandler("start"));
    edge(BUTTON.START, startHandler);
    return dispatched;
  }

  private findFallbackHandler(key: keyof GamepadScopeHandlers): (() => void) | undefined {
    const eligible = this.scopes.filter((candidate) =>
      candidate.enabled && !candidate.paused && !candidate.overlay && typeof candidate.handlers[key] === "function"
    );
    if (eligible.length === 0) return undefined;
    return this.highestRanked(eligible).handlers[key] as (() => void) | undefined;
  }

  private setInputMode(mode: GamepadInputMode): void {
    if (this.inputMode === mode) return;
    this.inputMode = mode;
    for (const listener of this.modeListeners) listener(mode);
  }

  private resetInputState(): void {
    for (const direction of DIRECTIONS) {
      this.directionState[direction].held = false;
      this.directionState[direction].nextAt = 0;
    }
    this.buttonState.clear();
    this.axisStates.clear();
    this.triggerStates.clear();
  }
}

export function createGamepadFocusRuntime(
  environment: GamepadRuntimeEnvironment,
  options: GamepadRuntimeOptions = {},
): GamepadFocusRuntime {
  return new GamepadFocusRuntime(environment, options);
}

let defaultRuntime: GamepadFocusRuntime | null | undefined;

export function getDefaultGamepadFocusRuntime(): GamepadFocusRuntime | null {
  if (defaultRuntime !== undefined) return defaultRuntime;
  const environment = defaultEnvironment();
  defaultRuntime = environment ? new GamepadFocusRuntime(environment) : null;
  return defaultRuntime;
}

/** Test hook; production code should use the lazy default runtime. */
export function setDefaultGamepadFocusRuntimeForTesting(runtime: GamepadFocusRuntime | null | undefined): void {
  if (defaultRuntime && defaultRuntime !== runtime) defaultRuntime.destroy();
  defaultRuntime = runtime;
}

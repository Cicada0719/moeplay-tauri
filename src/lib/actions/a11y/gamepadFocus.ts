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
  VIEW: 8,
  START: 9,
  DPAD_UP: 12,
  DPAD_DOWN: 13,
  DPAD_LEFT: 14,
  DPAD_RIGHT: 15,
} as const;

const DIRECTIONS: readonly GamepadDirection[] = ["up", "down", "left", "right"];
const EDGE_BUTTONS = [BUTTON.A, BUTTON.B, BUTTON.X, BUTTON.Y, BUTTON.LB, BUTTON.RB, BUTTON.VIEW, BUTTON.START] as const;
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
  private activeZone: GamepadZone | null = null;
  private activeScopeId: string | null = null;
  private inputMode: GamepadInputMode = "keyboard";
  private frameHandle: number | null = null;
  private running = false;
  private globallyPaused = false;
  private listenersInstalled = false;
  private awaitingNeutralAfterKeyboard = false;
  private awaitingNeutralAfterScopeChange = false;
  private horizontalAxis: -1 | 0 | 1 = 0;
  private verticalAxis: -1 | 0 | 1 = 0;
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
    const pad = this.findPad();
    this.awaitingNeutralAfterKeyboard = pad ? !this.sampleIsNeutral(this.readSample(pad)) : false;
    this.resetInputState();
  }

  /** Poll once. Public for deterministic tests and non-RAF hosts. */
  poll(now = this.environment.clock.now()): void {
    if (this.globallyPaused || this.scopes.length === 0) return;
    if (this.environment.hasFocus && !this.environment.hasFocus()) {
      this.resetInputState();
      return;
    }

    const pad = this.findPad();
    if (!pad) {
      this.resetInputState();
      return;
    }

    const sample = this.readSample(pad);
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

  private findPad(): GamepadLike | null {
    try {
      return Array.from(this.environment.navigator.getGamepads() ?? [])
        .find((pad): pad is GamepadLike => pad != null && pad.connected !== false) ?? null;
    } catch {
      return null;
    }
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
    if (current === -1 && value <= -this.axisReleaseThreshold) return -1;
    if (current === 1 && value >= this.axisReleaseThreshold) return 1;
    if (value <= -this.axisPressThreshold) return -1;
    if (value >= this.axisPressThreshold) return 1;
    return 0;
  }

  private readSample(pad: GamepadLike): InputSample {
    this.horizontalAxis = this.readAxis(Number(pad.axes[0] ?? 0), this.horizontalAxis);
    this.verticalAxis = this.readAxis(Number(pad.axes[1] ?? 0), this.verticalAxis);

    let left = safePressed(pad.buttons, BUTTON.DPAD_LEFT) || this.horizontalAxis === -1;
    let right = safePressed(pad.buttons, BUTTON.DPAD_RIGHT) || this.horizontalAxis === 1;
    let up = safePressed(pad.buttons, BUTTON.DPAD_UP) || this.verticalAxis === -1;
    let down = safePressed(pad.buttons, BUTTON.DPAD_DOWN) || this.verticalAxis === 1;

    if (left && right) left = right = false;
    if (up && down) up = down = false;

    return {
      directions: { up, down, left, right } satisfies Record<GamepadDirection, boolean>,
      buttons: new Map<number, boolean>(EDGE_BUTTONS.map((index) => [index, safePressed(pad.buttons, index)])),
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
        nextAt: held ? now + this.initialRepeatDelayMs : 0,
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
        state.nextAt = now + this.initialRepeatDelayMs;
        scope.handlers[direction]?.();
        dispatched = true;
      } else if (now >= state.nextAt) {
        state.nextAt += this.repeatIntervalMs;
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
    this.horizontalAxis = 0;
    this.verticalAxis = 0;
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

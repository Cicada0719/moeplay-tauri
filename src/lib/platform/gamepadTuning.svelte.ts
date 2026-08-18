// 手柄灵敏度调参（串流/远程场景适配）：摇杆死区与连发速度可调。
// 默认值与旧版完全一致（标准：0.55/0.35 阈值、100ms 连发、320ms 首延迟），
// 仅在用户修改时生效；runtime 通过 revision 缓存避免逐帧读 localStorage。

export type AxisSensitivity = "loose" | "standard" | "tight";
export type RepeatSpeed = "slow" | "standard" | "fast";

const SENSITIVITY_KEY = "moeplay-gamepad-sensitivity-v1";
const REPEAT_KEY = "moeplay-gamepad-repeat-v1";

export const AXIS_PRESETS: Record<AxisSensitivity, { press: number; release: number }> = {
  loose: { press: 0.7, release: 0.45 },
  standard: { press: 0.55, release: 0.35 },
  tight: { press: 0.4, release: 0.25 },
};

export const REPEAT_PRESETS: Record<RepeatSpeed, number> = {
  slow: 160,
  standard: 100,
  fast: 60,
};

function readPref<T extends string>(key: string, allowed: readonly T[], fallback: T): T {
  if (typeof localStorage === "undefined") return fallback;
  const raw = localStorage.getItem(key);
  return (allowed as readonly string[]).includes(raw ?? "") ? (raw as T) : fallback;
}

let _sensitivity = $state<AxisSensitivity>(readPref(SENSITIVITY_KEY, ["loose", "standard", "tight"] as const, "standard"));
let _repeatSpeed = $state<RepeatSpeed>(readPref(REPEAT_KEY, ["slow", "standard", "fast"] as const, "standard"));
let tuningRevision = 0;

export function getGamepadTuningRevision(): number {
  return tuningRevision;
}

export const gamepadTuning = {
  get sensitivity() { return _sensitivity; },
  set sensitivity(value: AxisSensitivity) {
    _sensitivity = value;
    if (typeof localStorage !== "undefined") localStorage.setItem(SENSITIVITY_KEY, value);
    tuningRevision += 1;
  },
  get repeatSpeed() { return _repeatSpeed; },
  set repeatSpeed(value: RepeatSpeed) {
    _repeatSpeed = value;
    if (typeof localStorage !== "undefined") localStorage.setItem(REPEAT_KEY, value);
    tuningRevision += 1;
  },
  get axisPress() { return AXIS_PRESETS[_sensitivity].press; },
  get axisRelease() { return AXIS_PRESETS[_sensitivity].release; },
  get repeatIntervalMs() { return REPEAT_PRESETS[_repeatSpeed]; },
  /** 首按延迟跟随连发速度（标准 320ms，与历史默认一致） */
  get initialDelayMs() { return Math.round(REPEAT_PRESETS[_repeatSpeed] * 3.2); },
};

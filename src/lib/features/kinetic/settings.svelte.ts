/**
 * 「电影化主视觉」开关 —— 纯前端 localStorage 层扩展。
 *
 * 参照 moeplay-appearance-v1 的既有模式：版本化 key + JSON payload +
 * try/catch 容错（隐私模式/损坏数据回退默认）。不触碰 Rust settings
 * schema 与 api/types.ts；读取端为本模块与 SwitchHome，写入端为设置页。
 */

export const KINETIC_STAGE_STORAGE_KEY = "moeplay-kinetic-stage-v1";

interface KineticStageStoragePayload {
  enabled?: unknown;
}

function readStoredEnabled(): boolean {
  if (typeof localStorage === "undefined") return true;
  try {
    const stored = localStorage.getItem(KINETIC_STAGE_STORAGE_KEY);
    if (!stored) return true;
    const payload = JSON.parse(stored) as KineticStageStoragePayload | null;
    // 默认开：仅当显式写入 false 时关闭。
    return payload?.enabled !== false;
  } catch {
    return true;
  }
}

export function readKineticStageEnabled(): boolean {
  return readStoredEnabled();
}

export function writeKineticStageEnabled(enabled: boolean): void {
  if (typeof localStorage === "undefined") return;
  try {
    localStorage.setItem(KINETIC_STAGE_STORAGE_KEY, JSON.stringify({ enabled } satisfies KineticStageStoragePayload));
  } catch {
    /* private mode */
  }
}

let enabled = $state(readStoredEnabled());

/** 共享响应式开关状态；设置页写入后 SwitchHome 无需刷新即可响应。 */
export const kineticStageStore = {
  get enabled() {
    return enabled;
  },
  setEnabled(value: boolean) {
    enabled = value;
    writeKineticStageEnabled(value);
  },
};

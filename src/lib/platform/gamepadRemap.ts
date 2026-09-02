// 手柄按键绑定（重映射）：把语义动作绑定到任意物理按钮索引。
// 不重绑时沿用「手柄按键布局」的默认映射（Xbox/W3C 语义；任天堂布局自动换位）。
// 显式重绑为物理索引，不受布局换位影响。
import { mapFaceButton, type GamepadLayout } from "./gamepadLayout";

export type GamepadAction =
  | "launch"
  | "back"
  | "favorite"
  | "activate"
  | "pageLeft"
  | "pageRight"
  | "categoryLeft"
  | "categoryRight"
  | "filter"
  | "start";

export type GamepadRemap = Partial<Record<GamepadAction, number>>;

const REMAP_STORAGE_KEY = "moeplay-gamepad-remap-v1";

/** 语义面键索引 → 动作名（方向键/摇杆不参与重绑） */
export const ACTION_BY_SEMANTIC: Record<number, GamepadAction> = {
  0: "launch",
  1: "back",
  2: "favorite",
  3: "activate",
  4: "pageLeft",
  5: "pageRight",
  6: "categoryLeft",
  7: "categoryRight",
  8: "filter",
  9: "start",
};

export const GAMEPAD_ACTIONS: GamepadAction[] = [
  "launch",
  "back",
  "favorite",
  "activate",
  "pageLeft",
  "pageRight",
  "categoryLeft",
  "categoryRight",
  "filter",
  "start",
];

/** Xbox 语义的物理按钮标签（索引 → 名称） */
const XBOX_LABELS: Record<number, string> = {
  0: "A", 1: "B", 2: "X", 3: "Y", 4: "LB", 5: "RB", 6: "LT", 7: "RT", 8: "VIEW", 9: "START",
};

/** 任天堂布局下的物理按钮标签（A 在右、B 在下、X 在上、Y 在左） */
const NINTENDO_LABELS: Record<number, string> = {
  0: "B", 1: "A", 2: "Y", 3: "X", 4: "LB", 5: "RB", 6: "ZL", 7: "ZR", 8: "VIEW", 9: "START",
};

/** PlayStation 布局下的物理按钮标签（下 ✕ / 右 ○ / 左 □ / 上 △） */
const PLAYSTATION_LABELS: Record<number, string> = {
  0: "✕", 1: "○", 2: "□", 3: "△", 4: "L1", 5: "R1", 6: "L2", 7: "R2", 8: "SHARE", 9: "OPTIONS",
};

let remapRevision = 0;

export function getGamepadRemapRevision(): number {
  return remapRevision;
}

/** 读取按键绑定表（非法值回退空表，不抛错） */
export function readGamepadRemap(): GamepadRemap {
  if (typeof localStorage === "undefined") return {};
  try {
    const raw = localStorage.getItem(REMAP_STORAGE_KEY);
    if (!raw) return {};
    const parsed: unknown = JSON.parse(raw);
    if (typeof parsed !== "object" || parsed === null || Array.isArray(parsed)) return {};
    const out: GamepadRemap = {};
    for (const [action, value] of Object.entries(parsed as Record<string, unknown>)) {
      if (
        (GAMEPAD_ACTIONS as string[]).includes(action) &&
        typeof value === "number" &&
        Number.isInteger(value) &&
        value >= 0 && value <= 17
      ) {
        out[action as GamepadAction] = value;
      }
    }
    return out;
  } catch {
    return {};
  }
}

/** 写入按键绑定表 */
export function writeGamepadRemap(remap: GamepadRemap): void {
  if (typeof localStorage === "undefined") return;
  localStorage.setItem(REMAP_STORAGE_KEY, JSON.stringify(remap));
  remapRevision += 1;
}

/** 恢复默认绑定 */
export function resetGamepadRemap(): void {
  if (typeof localStorage === "undefined") return;
  localStorage.removeItem(REMAP_STORAGE_KEY);
  remapRevision += 1;
}

/** 语义面键索引 → 最终物理按钮索引（显式重绑优先，否则按布局默认换位） */
export function physicalButtonFor(semanticIndex: number, layout: GamepadLayout): number {
  const action = ACTION_BY_SEMANTIC[semanticIndex];
  if (!action) return semanticIndex;
  const explicit = readGamepadRemap()[action];
  if (explicit !== undefined) return explicit;
  return mapFaceButton(layout, semanticIndex);
}

/** 动作当前绑定的物理按钮标签（用于提示条与设置页展示） */
export function gamepadGlyphFor(action: GamepadAction, layout: GamepadLayout): string {
  const explicit = readGamepadRemap()[action];
  const physical = explicit !== undefined
    ? explicit
    : mapFaceButton(layout, ACTION_SEMANTIC_BY_ACTION[action]);
  const labels = layout === "nintendo" ? NINTENDO_LABELS : layout === "playstation" ? PLAYSTATION_LABELS : XBOX_LABELS;
  return labels[physical] ?? String(physical);
}

const ACTION_SEMANTIC_BY_ACTION: Record<GamepadAction, number> = {
  launch: 0,
  back: 1,
  favorite: 2,
  activate: 3,
  pageLeft: 4,
  pageRight: 5,
  categoryLeft: 6,
  categoryRight: 7,
  filter: 8,
  start: 9,
};

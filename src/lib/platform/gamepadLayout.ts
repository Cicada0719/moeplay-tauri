// 手柄按键布局：Xbox（W3C standard mapping 语义）与任天堂（A 在右、B 在下）的面键
// 位置互换。采用 label-based 映射：任天堂布局下 A=确认、B=取消（任天堂习惯），
// 因此提示条文案（A 确认 / B 返回）天然成立，无需改动。

export type GamepadLayout = "xbox" | "nintendo";
export type GamepadLayoutPreference = "auto" | GamepadLayout;

const LAYOUT_STORAGE_KEY = "moeplay-gamepad-layout-v1";

// Switch Pro / Joy-Con / 第三方 Switch 协议手柄（含 vendor 057e）的 id 特征
const NINTENDO_PATTERN = /nintendo|pro controller|joy-?con|057e/i;

/** 按手柄 id 判定布局；非任天堂特征一律按 Xbox/W3C 语义处理 */
export function detectGamepadLayout(id: string): GamepadLayout {
  return NINTENDO_PATTERN.test(id) ? "nintendo" : "xbox";
}

/** 结合用户偏好（auto 时按 id 检测）解析最终布局 */
export function resolveGamepadLayout(
  id: string,
  override: GamepadLayoutPreference = "auto",
): GamepadLayout {
  return override === "auto" ? detectGamepadLayout(id) : override;
}

/**
 * 语义面键索引 → 物理按钮索引。
 * 运行时的语义常量固定为 Xbox/W3C（0=下/确认、1=右/取消、2=左、3=上）；
 * 任天堂布局下确认换到右键（A）、取消换到下键（B），即 0↔1、2↔3 互换。
 * 方向键、LB/RB、VIEW、START 不经过此函数，保持原索引。
 */
export function mapFaceButton(layout: GamepadLayout, semanticIndex: number): number {
  if (layout !== "nintendo") return semanticIndex;
  switch (semanticIndex) {
    case 0: return 1;
    case 1: return 0;
    case 2: return 3;
    case 3: return 2;
    default: return semanticIndex;
  }
}

/** 读取用户布局偏好（auto/xbox/nintendo），缺失或非法值回退 auto */
export function readGamepadLayoutPreference(): GamepadLayoutPreference {
  if (typeof localStorage === "undefined") return "auto";
  const raw = localStorage.getItem(LAYOUT_STORAGE_KEY);
  return raw === "xbox" || raw === "nintendo" ? raw : "auto";
}

/** 写入用户布局偏好 */
export function writeGamepadLayoutPreference(value: GamepadLayoutPreference): void {
  if (typeof localStorage === "undefined") return;
  localStorage.setItem(LAYOUT_STORAGE_KEY, value);
}

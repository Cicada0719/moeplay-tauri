// 手柄按键布局：Xbox（W3C standard mapping 语义）与任天堂（A 在右、B 在下）的面键
// 位置互换。采用 label-based 映射：任天堂布局下 A=确认、B=取消（任天堂习惯），
// 因此提示条文案（A 确认 / B 返回）天然成立，无需改动。
//
// 布局解析优先级（低 → 高）：
//   自动检测（id 特征）→ 全局偏好（设置页「手柄按键布局」）→ 逐手柄覆盖（设置页「已连接手柄」）
// 逐手柄覆盖按「id#槽位」记忆：串流/虚拟手柄工具（UU远程 等）上报的 id 可能与
// 真实手柄完全相同（例如虚拟设备也报 "Xbox 360 Controller (XInput STANDARD GAMEPAD)"），
// 仅凭 id 无法区分远端实体手柄类型，因此必须允许用户为每个设备单独指定布局。

export type GamepadLayout = "xbox" | "nintendo" | "playstation";
export type GamepadLayoutPreference = "auto" | GamepadLayout;

const LAYOUT_STORAGE_KEY = "moeplay-gamepad-layout-v1";
const DEVICE_LAYOUT_STORAGE_KEY = "moeplay-gamepad-layout-devices-v1";

// Switch Pro / Joy-Con / 第三方 Switch 协议手柄（含 vendor 057e）的 id 特征
const NINTENDO_PATTERN = /nintendo|pro controller|joy-?con|057e/i;
// PlayStation 手柄（DualShock/DualSense，含 vendor 054c）的 id 特征
const PLAYSTATION_PATTERN = /playstation|dualshock|dualsense|054c|ps4|ps5/i;

// 布局配置版本号：每次写入全局偏好或逐手柄覆盖时递增。
// gamepadFocus runtime 用它判断缓存是否失效（避免逐帧读 localStorage）。
let layoutRevision = 0;

export function getGamepadLayoutRevision(): number {
  return layoutRevision;
}

/** 按手柄 id 判定布局：任天堂 > PlayStation > Xbox/W3C */
export function detectGamepadLayout(id: string): GamepadLayout {
  if (NINTENDO_PATTERN.test(id)) return "nintendo";
  if (PLAYSTATION_PATTERN.test(id)) return "playstation";
  return "xbox";
}

/** 逐手柄覆盖条目的存储键：优先「id#槽位」，无槽位信息时退回「id」 */
export function deviceLayoutKeyFor(id: string, index?: number): string {
  const normalized = id.trim();
  return typeof index === "number" ? normalized + "#" + index : normalized;
}

/** 读取逐手柄布局覆盖表（非法内容一律回退空表，不抛错） */
export function readDeviceLayoutMap(): Record<string, GamepadLayout> {
  if (typeof localStorage === "undefined") return {};
  try {
    const raw = localStorage.getItem(DEVICE_LAYOUT_STORAGE_KEY);
    if (!raw) return {};
    const parsed: unknown = JSON.parse(raw);
    if (typeof parsed !== "object" || parsed === null || Array.isArray(parsed)) return {};
    const out: Record<string, GamepadLayout> = {};
    for (const [key, value] of Object.entries(parsed as Record<string, unknown>)) {
      if (value === "xbox" || value === "nintendo") out[key] = value;
    }
    return out;
  } catch {
    return {};
  }
}

/** 写入/清除某个手柄的布局覆盖；传 null 清除该条目 */
export function writeDeviceLayoutPreference(id: string, layout: GamepadLayout | null, index?: number): void {
  if (typeof localStorage === "undefined") return;
  const map = readDeviceLayoutMap();
  const key = deviceLayoutKeyFor(id, index);
  if (layout === null) delete map[key];
  else map[key] = layout;
  localStorage.setItem(DEVICE_LAYOUT_STORAGE_KEY, JSON.stringify(map));
  layoutRevision += 1;
}

/** 查询某个手柄的布局覆盖（无覆盖返回 null） */
export function readDeviceLayoutFor(id: string, index?: number): GamepadLayout | null {
  const map = readDeviceLayoutMap();
  if (typeof index === "number") {
    const specific = map[deviceLayoutKeyFor(id, index)];
    if (specific) return specific;
  }
  return map[deviceLayoutKeyFor(id)] ?? null;
}

/** 结合逐手柄覆盖与全局偏好（auto 时按 id 检测）解析最终布局 */
export function resolveGamepadLayout(
  id: string,
  override: GamepadLayoutPreference = "auto",
  index?: number,
): GamepadLayout {
  const deviceLayout = readDeviceLayoutFor(id, index);
  if (deviceLayout) return deviceLayout;
  return override === "auto" ? detectGamepadLayout(id) : override;
}

/**
 * 语义面键索引 → 物理按钮索引。
 * 运行时的语义常量固定为 Xbox/W3C（0=下/确认、1=右/取消、2=左、3=上）；
 * 任天堂布局下确认换到右键（A）、取消换到下键（B），即 0↔1、2↔3 互换。
 * 方向键、LB/RB、VIEW、START 不经过此函数，保持原索引。
 */
export function mapFaceButton(layout: GamepadLayout, semanticIndex: number): number {
  // PlayStation 物理布局（下 ✕/右 ○/左 □/上 △）与 Xbox/W3C 语义顺序一致，无需换位
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
  return raw === "xbox" || raw === "nintendo" || raw === "playstation" ? raw : "auto";
}

/** 写入用户布局偏好 */
export function writeGamepadLayoutPreference(value: GamepadLayoutPreference): void {
  if (typeof localStorage === "undefined") return;
  localStorage.setItem(LAYOUT_STORAGE_KEY, value);
  layoutRevision += 1;
}

/** 已连接手柄的最小描述（供设置页/提示条渲染，避免直接依赖浏览器 Gamepad 类型） */
export interface ConnectedPadLike {
  id: string;
  index: number;
  connected: boolean;
}

/** 解析一组已连接手柄各自的最终布局（逐手柄覆盖 → 全局偏好 → 自动检测） */
export function resolveConnectedPadLayouts(
  pads: ArrayLike<ConnectedPadLike | null>,
  override: GamepadLayoutPreference = readGamepadLayoutPreference(),
): { id: string; index: number; layout: GamepadLayout; detected: GamepadLayout; deviceOverride: GamepadLayout | null }[] {
  return Array.from(pads ?? [])
    .filter((pad): pad is ConnectedPadLike => pad != null && pad.connected)
    .map((pad) => {
      const index = typeof pad.index === "number" ? pad.index : undefined;
      return {
        id: pad.id,
        index: typeof pad.index === "number" ? pad.index : 0,
        layout: resolveGamepadLayout(pad.id, override, index),
        detected: detectGamepadLayout(pad.id),
        deviceOverride: readDeviceLayoutFor(pad.id, index),
      };
    });
}

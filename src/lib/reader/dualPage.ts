// 双页配对纯函数模块（FR-11 / 风险 R8）
//
// 核心架构约束：**页索引以单页为原子单位存储（与渲染模式无关），双页仅为视图层配对**。
// 本文件只做配对 / 定位 / 翻页的纯计算，不依赖 DOM / Svelte，可被 vitest 直接单测。
// 方向（LTR/RTL）只影响 `intentFromInput` 的物理输入映射，不参与配对本身。

/** 页面元信息（仅需宽高，由图片预加载后填充；未知时按普通页处理） */
export interface PageMeta {
  /** 单页序号，0-based，全局原子单位 */
  index: number;
  width?: number;
  height?: number;
}

/** 一屏（一次渲染单元）包含 1 或 2 个页面 */
export interface Screen {
  /** 长度 1（跨页大图独占）或 2 */
  pageIndexes: number[];
  /** 该屏的"锚定页"，用于历史记录与恢复定位（取 pageIndexes 最小值） */
  anchorIndex: number;
}

/**
 * 是否跨页大图：宽 > 高 × 1.5。
 * 任一维度缺失 / 非法时按普通页处理（返回 false）。
 */
export function isSpreadImage(width: number, height: number): boolean {
  if (!Number.isFinite(width) || !Number.isFinite(height)) return false;
  if (width <= 0 || height <= 0) return false;
  return width > height * 1.5;
}

/**
 * 计算双页模式下的屏幕序列（纯函数）。
 * 规则：
 *  - 跨页大图独占一屏；
 *  - 其余页两两配对；
 *  - 最后一页落单时生成单元素 Screen；
 *  - 配对不受方向（LTR/RTL）影响，方向仅影响渲染顺序与翻页映射（R8 约束）。
 */
export function buildScreens(pages: PageMeta[]): Screen[] {
  const screens: Screen[] = [];
  const buffer: number[] = [];

  const flush = () => {
    if (buffer.length === 0) return;
    screens.push({
      pageIndexes: [...buffer],
      anchorIndex: buffer[0],
    });
    buffer.length = 0;
  };

  for (const page of pages) {
    if (isSpreadImage(page.width ?? 0, page.height ?? 0)) {
      // 跨页大图独占一屏：先把累积的普通页落屏，再独占一屏。
      flush();
      screens.push({ pageIndexes: [page.index], anchorIndex: page.index });
    } else {
      buffer.push(page.index);
      if (buffer.length === 2) flush();
    }
  }
  flush();
  return screens;
}

/**
 * 给定当前单页页索引，返回其所在屏幕的下标；用于从历史恢复定位。
 * 找不到时抛 `RangeError`（越界 index 属调用方错误，不应静默吞掉）。
 */
export function screenIndexOfPage(screens: Screen[], pageIndex: number): number {
  for (let index = 0; index < screens.length; index += 1) {
    if (screens[index].pageIndexes.includes(pageIndex)) return index;
  }
  throw new RangeError(
    `screenIndexOfPage: page ${pageIndex} not found in ${screens.length} screens`,
  );
}

/**
 * 翻页映射。
 * @param intent 用户意图：'forward'(内容前进) | 'backward'(内容后退)
 * @returns 目标屏幕下标（越界时返回 clamp 后的边界值）
 */
export function nextScreen(
  screens: Screen[],
  current: number,
  intent: 'forward' | 'backward',
): number {
  if (screens.length === 0) return 0;
  const raw = intent === 'forward' ? current + 1 : current - 1;
  return Math.min(screens.length - 1, Math.max(0, raw));
}

/**
 * 将"物理方向输入"（点击左/右区域、按 ←/→ 键）翻译为内容意图。
 * RTL：右区域 / → 键 = forward；左区域 / ← 键 = backward。
 * LTR：左区域 / ← 键 = backward；右区域 / → 键 = forward。
 */
export function intentFromInput(
  physical: 'left' | 'right',
  direction: 'ltr' | 'rtl',
): 'forward' | 'backward' {
  // RTL 下 右侧=forward、左侧=backward；LTR 下互为镜像。
  const forward = direction === 'rtl' ? physical === 'right' : physical === 'left';
  return forward ? 'forward' : 'backward';
}

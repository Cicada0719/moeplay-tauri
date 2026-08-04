// dualPage.ts 纯函数单测（spec §6.1 U1~U9，覆盖率 ≥90%）
import { describe, expect, it } from "vitest";
import {
  buildScreens,
  intentFromInput,
  isSpreadImage,
  nextScreen,
  screenIndexOfPage,
  type PageMeta,
} from "../dualPage";

function page(index: number, width?: number, height?: number): PageMeta {
  return { index, width, height };
}

describe("isSpreadImage（U1 边界）", () => {
  it("宽 > 高×1.5 → true", () => {
    expect(isSpreadImage(2000, 1000)).toBe(true);
  });
  it("高大于宽 → false", () => {
    expect(isSpreadImage(1000, 2000)).toBe(false);
  });
  it("恰好 1.5 倍 → false（严格大于）", () => {
    expect(isSpreadImage(1500, 1000)).toBe(false);
  });
  it("宽高缺失 / 非法 → false（按普通页处理）", () => {
    expect(isSpreadImage(0, 0)).toBe(false);
    expect(isSpreadImage(Number.NaN, 100)).toBe(false);
    expect(isSpreadImage(-100, 100)).toBe(false);
  });
});

describe("buildScreens（U2/U3/U4/U5/U9）", () => {
  it("10 个普通页 → 5 屏，每屏 2 页，anchorIndex=[0,2,4,6,8]", () => {
    const pages = Array.from({ length: 10 }, (_, index) => page(index, 100, 100));
    const screens = buildScreens(pages);
    expect(screens).toHaveLength(5);
    expect(screens.map((screen) => screen.anchorIndex)).toEqual([0, 2, 4, 6, 8]);
    expect(screens.every((screen) => screen.pageIndexes.length === 2)).toBe(true);
    expect(screens[1].pageIndexes).toEqual([2, 3]);
  });

  it("9 个普通页 → 5 屏，末屏单页（落单）", () => {
    const pages = Array.from({ length: 9 }, (_, index) => page(index, 100, 100));
    const screens = buildScreens(pages);
    expect(screens).toHaveLength(5);
    expect(screens.at(-1)!.pageIndexes).toEqual([8]);
    expect(screens.at(-1)!.anchorIndex).toBe(8);
  });

  it("[普通, 跨页, 普通, 普通] → [0], [1], [2,3]", () => {
    const pages = [
      page(0, 100, 100),
      page(1, 2000, 1000),
      page(2, 100, 100),
      page(3, 100, 100),
    ];
    const screens = buildScreens(pages);
    expect(screens.map((screen) => screen.pageIndexes)).toEqual([[0], [1], [2, 3]]);
  });

  it("连续两张跨页图各自独占一屏", () => {
    const pages = [page(0, 2000, 1000), page(1, 1800, 900)];
    const screens = buildScreens(pages);
    expect(screens.map((screen) => screen.pageIndexes)).toEqual([[0], [1]]);
  });

  it("空数组 → []（U9）", () => {
    expect(buildScreens([])).toEqual([]);
  });

  it("配对与 direction 无关：同样的页序列在 LTR/RTL 语义下结果一致（R8）", () => {
    const pages = Array.from({ length: 7 }, (_, index) => page(index, 100, 100));
    expect(buildScreens(pages)).toEqual(buildScreens(pages));
  });
});

describe("screenIndexOfPage（U6/U9 异常）", () => {
  it("命中配对屏返回其下标", () => {
    const screens = buildScreens(
      Array.from({ length: 10 }, (_, index) => page(index, 100, 100)),
    );
    expect(screenIndexOfPage(screens, 7)).toBe(3); // [6,7]
    expect(screenIndexOfPage(screens, 0)).toBe(0);
    expect(screenIndexOfPage(screens, 9)).toBe(4);
  });

  it("越界 index 抛 RangeError", () => {
    const screens = buildScreens(
      Array.from({ length: 4 }, (_, index) => page(index, 100, 100)),
    );
    expect(() => screenIndexOfPage(screens, 99)).toThrow(RangeError);
    expect(() => screenIndexOfPage([], 0)).toThrow(RangeError);
  });
});

describe("nextScreen（U7 clamp）", () => {
  it("0 处 backward 返回 0（clamp）", () => {
    const screens = buildScreens(
      Array.from({ length: 4 }, (_, index) => page(index, 100, 100)),
    );
    expect(nextScreen(screens, 0, "backward")).toBe(0);
  });

  it("末屏 forward 返回末屏（clamp）", () => {
    const screens = buildScreens(
      Array.from({ length: 4 }, (_, index) => page(index, 100, 100)),
    );
    expect(nextScreen(screens, screens.length - 1, "forward")).toBe(screens.length - 1);
  });

  it("中间位置前进/后退各移一屏", () => {
    const screens = buildScreens(
      Array.from({ length: 8 }, (_, index) => page(index, 100, 100)),
    );
    expect(nextScreen(screens, 1, "forward")).toBe(2);
    expect(nextScreen(screens, 2, "backward")).toBe(1);
  });

  it("空屏序列返回 0", () => {
    expect(nextScreen([], 0, "forward")).toBe(0);
  });
});

describe("intentFromInput（U8 四组合 + 镜像）", () => {
  it("RTL：右区域 = forward，左区域 = backward", () => {
    expect(intentFromInput("right", "rtl")).toBe("forward");
    expect(intentFromInput("left", "rtl")).toBe("backward");
  });

  it("LTR：左区域 = forward，右区域 = backward（互为镜像）", () => {
    expect(intentFromInput("left", "ltr")).toBe("forward");
    expect(intentFromInput("right", "ltr")).toBe("backward");
  });
});

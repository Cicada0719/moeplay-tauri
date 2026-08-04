// 预取 LRU 缓存模块测试（spec §4 步骤 5.6：模块级 Map + width*height*4 估算 + >200MB 淘汰）
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  MAX_PRELOAD_BYTES,
  attachPreloadImage,
  clearPreloadCache,
  getPreloadBytes,
  getPreloadEntry,
  getPreloadSize,
  hasPreload,
  touchPreload,
} from "./preloadCache";

const MB = 1024 * 1024;

beforeEach(() => {
  vi.useFakeTimers();
  clearPreloadCache();
});

afterEach(() => {
  clearPreloadCache();
  vi.useRealTimers();
});

describe("preloadCache LRU 淘汰", () => {
  it("累计超 200MB 时淘汰最久未用项，preloadBytes 随之扣减", () => {
    vi.setSystemTime(1000);
    touchPreload("a", 150 * MB);
    vi.setSystemTime(2000);
    touchPreload("b", 60 * MB); // 210MB > 200MB → 淘汰 a

    expect(hasPreload("a")).toBe(false);
    expect(hasPreload("b")).toBe(true);
    expect(getPreloadBytes()).toBe(60 * MB);
    expect(getPreloadSize()).toBe(1);
  });

  it("累计恰为 200MB 时不淘汰（仅 > 上限才触发）", () => {
    touchPreload("a", 100 * MB);
    touchPreload("b", 100 * MB);

    expect(getPreloadBytes()).toBe(200 * MB);
    expect(hasPreload("a")).toBe(true);
    expect(hasPreload("b")).toBe(true);
    expect(getPreloadSize()).toBe(2);
  });

  it("一次超额可连续淘汰多项直到 ≤ 上限", () => {
    vi.setSystemTime(1000);
    touchPreload("a", 80 * MB);
    vi.setSystemTime(2000);
    touchPreload("b", 80 * MB);
    vi.setSystemTime(3000);
    touchPreload("c", 80 * MB); // 240MB → 淘汰 a → 160MB 停
    expect(hasPreload("a")).toBe(false);
    expect(hasPreload("b")).toBe(true);
    expect(hasPreload("c")).toBe(true);
    expect(getPreloadBytes()).toBe(160 * MB);

    vi.setSystemTime(4000);
    touchPreload("d", 160 * MB); // 160+160=320 → 连续淘汰 b(80)、c(80) → 160MB 停
    expect(hasPreload("b")).toBe(false);
    expect(hasPreload("c")).toBe(false);
    expect(hasPreload("d")).toBe(true);
    expect(getPreloadBytes()).toBe(160 * MB);
  });

  it("re-touch 更新 lastUsed：最新触碰的条目不被淘汰", () => {
    vi.setSystemTime(1000);
    touchPreload("a", 150 * MB);
    vi.setSystemTime(2000);
    touchPreload("b", 60 * MB); // 210MB → 淘汰 a → 剩 b
    expect(hasPreload("a")).toBe(false);

    vi.setSystemTime(3000);
    touchPreload("a", 150 * MB); // b(60)+a(150)=210 → 淘汰 b（a 刚触碰最新）
    expect(hasPreload("b")).toBe(false);
    expect(hasPreload("a")).toBe(true);
    expect(getPreloadBytes()).toBe(150 * MB);
  });
});

describe("preloadCache 字节回填", () => {
  it("预取时 bytes=0，探测到尺寸后回填补账并触发淘汰", () => {
    vi.setSystemTime(1000);
    touchPreload("a"); // 未探测 → 0
    expect(getPreloadEntry("a")!.bytes).toBe(0);
    expect(getPreloadBytes()).toBe(0);

    vi.setSystemTime(2000);
    touchPreload("a", 150 * MB); // 回填 150MB
    expect(getPreloadEntry("a")!.bytes).toBe(150 * MB);
    expect(getPreloadBytes()).toBe(150 * MB);

    vi.setSystemTime(3000);
    touchPreload("b", 60 * MB); // 210MB → 淘汰 a
    expect(hasPreload("a")).toBe(false);
    expect(hasPreload("b")).toBe(true);
    expect(getPreloadBytes()).toBe(60 * MB);
  });

  it("re-touch 已缓存条目：lastUsed 更新、bytes 与总量不变", () => {
    vi.setSystemTime(1000);
    touchPreload("a", 10 * MB);
    expect(getPreloadBytes()).toBe(10 * MB);

    vi.setSystemTime(2000);
    touchPreload("a"); // 不传 bytes，保留原有估算
    const entry = getPreloadEntry("a")!;
    expect(entry.lastUsed).toBe(2000);
    expect(entry.bytes).toBe(10 * MB);
    expect(getPreloadBytes()).toBe(10 * MB);
    expect(getPreloadSize()).toBe(1);
  });
});

describe("preloadCache Image 引用持有与释放", () => {
  it("预取 Image 引用存入 entry，淘汰时置 src='' 并删除引用", () => {
    const imgA = new Image();
    vi.setSystemTime(1000);
    touchPreload("a", 150 * MB);
    attachPreloadImage("a", imgA);
    expect(getPreloadEntry("a")!.img).toBe(imgA);

    vi.setSystemTime(2000);
    touchPreload("b", 60 * MB); // 210MB → 淘汰 a
    expect(imgA.src).toBe("");
    expect(getPreloadEntry("a")).toBeUndefined();
  });

  it("attachPreloadImage 替换旧引用时先释放旧 Image", () => {
    const imgOld = new Image();
    const imgNew = new Image();
    touchPreload("a", 10 * MB);
    attachPreloadImage("a", imgOld);
    attachPreloadImage("a", imgNew);

    expect(getPreloadEntry("a")!.img).toBe(imgNew);
    expect(imgOld.src).toBe("");
  });

  it("attachPreloadImage 对未缓存 URL 不创建条目（no-op）", () => {
    attachPreloadImage("nope", new Image());
    expect(hasPreload("nope")).toBe(false);
    expect(getPreloadSize()).toBe(0);
  });

  it("clearPreloadCache 释放全部 Image 引用并重置统计", () => {
    const imgA = new Image();
    touchPreload("a", 150 * MB);
    attachPreloadImage("a", imgA);
    touchPreload("b", 60 * MB);

    clearPreloadCache();
    expect(imgA.src).toBe("");
    expect(getPreloadSize()).toBe(0);
    expect(getPreloadBytes()).toBe(0);
  });
});

describe("preloadCache 常量契约", () => {
  it("上限为 200MB（spec §4 非功能需求）", () => {
    expect(MAX_PRELOAD_BYTES).toBe(200 * 1024 * 1024);
  });
});

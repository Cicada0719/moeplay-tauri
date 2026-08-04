// readerSettings.ts 单测（spec §6.1 U10~U13）
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  createReaderSettingsStore,
  DEFAULT_SETTINGS,
  GLOBAL_SETTINGS_KEY,
  loadSettings,
  mangaSettingsKey,
  saveSettings,
} from "./readerSettings";

beforeEach(() => {
  localStorage.clear();
  vi.restoreAllMocks();
});

describe("loadSettings / saveSettings", () => {
  it("U10：无存储时返回内置默认 {single, rtl, false}", () => {
    expect(loadSettings()).toEqual(DEFAULT_SETTINGS);
    expect(loadSettings("manga-a")).toEqual(DEFAULT_SETTINGS);
  });

  it("U11：单漫画覆盖优先于全局（全局 dual + 漫画A single）", () => {
    saveSettings(null, { pageMode: "dual" });
    saveSettings("manga-a", { pageMode: "single" });

    expect(loadSettings("manga-a").pageMode).toBe("single");
    expect(loadSettings("manga-a").direction).toBe("rtl"); // 未覆盖字段回落到全局/默认
    expect(loadSettings("manga-b").pageMode).toBe("dual");
  });

  it("U12：localStorage JSON 损坏 → 回退默认并 warn，不抛异常", () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    localStorage.setItem(GLOBAL_SETTINGS_KEY, "{ not valid json ");
    expect(loadSettings()).toEqual(DEFAULT_SETTINGS);
    expect(warn).toHaveBeenCalled();
  });

  it("U12b：损坏的单漫画键不污染全局读取", () => {
    vi.spyOn(console, "warn").mockImplementation(() => {});
    saveSettings(null, { pageMode: "dual" });
    localStorage.setItem(mangaSettingsKey("manga-a"), "oops");
    expect(loadSettings("manga-a").pageMode).toBe("dual"); // 全局兜底
  });

  it("saveSettings 对单漫画键写 Partial（merge 语义，不写死全局值）", () => {
    saveSettings("manga-a", { pageMode: "dual" });
    const raw = JSON.parse(localStorage.getItem(mangaSettingsKey("manga-a"))!);
    expect(raw).toEqual({ pageMode: "dual" }); // 只有被覆盖的字段
  });

  it("saveSettings 对全局键写完整 ReaderSettings（merge 语义）", () => {
    saveSettings(null, { direction: "ltr" });
    const raw = JSON.parse(localStorage.getItem(GLOBAL_SETTINGS_KEY)!);
    expect(raw.direction).toBe("ltr");
    expect(raw.pageMode).toBe("single");
    expect(raw.forceNarrowDual).toBe(false);
  });

  it("sanitize：写入非法枚举被丢弃", () => {
    saveSettings(null, { pageMode: "weird" as never });
    expect(loadSettings().pageMode).toBe("single");
  });
});

describe("createReaderSettingsStore", () => {
  it("U13：store update 后单漫画键被写入", () => {
    const store = createReaderSettingsStore("manga-a");
    store.update((settings) => ({ ...settings, direction: "ltr" }));
    const raw = JSON.parse(localStorage.getItem(mangaSettingsKey("manga-a"))!);
    expect(raw.direction).toBe("ltr");
  });

  it("store set 后键被写入完整值", () => {
    const store = createReaderSettingsStore("manga-b");
    store.set({ pageMode: "dual", direction: "ltr", forceNarrowDual: true });
    const raw = JSON.parse(localStorage.getItem(mangaSettingsKey("manga-b"))!);
    expect(raw).toEqual({ pageMode: "dual", direction: "ltr", forceNarrowDual: true });
  });

  it("创建时读取既有单漫画覆盖", () => {
    saveSettings("manga-c", { pageMode: "dual" });
    const store = createReaderSettingsStore("manga-c");
    let value: { pageMode: string } | undefined;
    store.subscribe((v) => (value = v))();
    expect(value?.pageMode).toBe("dual");
  });
});

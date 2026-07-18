import { describe, expect, it } from "vitest";
import { mergeKey, normalizeName } from "./normalize";

describe("anime-search normalizeName", () => {
  it("转小写并压缩首尾与连续空格", () => {
    expect(normalizeName("  Foo   Bar  ")).toBe("foo bar");
  });

  it("去除全角/半角括号及其内容", () => {
    expect(normalizeName("进击的巨人 [简体]")).toBe("进击的巨人");
    expect(normalizeName("名侦探柯南【高清】")).toBe("名侦探柯南");
    expect(normalizeName("某番 (2024)")).toBe("某番");
    expect(normalizeName("某番（OVA）")).toBe("某番");
    expect(normalizeName("[A] 某番 (B) 【C】")).toBe("某番");
  });

  it("去除中文季度/期数/部后缀", () => {
    expect(normalizeName("某番 第二季")).toBe("某番");
    expect(normalizeName("某番 第3期")).toBe("某番");
    expect(normalizeName("某番 第１０部")).toBe("某番");
    expect(normalizeName("某番第二季")).toBe("某番");
  });

  it("去除 S02 / Season / N期 后缀", () => {
    expect(normalizeName("某番 S02")).toBe("某番");
    expect(normalizeName("某番 s2")).toBe("某番");
    expect(normalizeName("某番 Season 2")).toBe("某番");
    expect(normalizeName("某番 2期")).toBe("某番");
  });

  it("去除分隔符 + 数字后缀（如 - 2）", () => {
    expect(normalizeName("某番 - 2")).toBe("某番");
    expect(normalizeName("某番: 2")).toBe("某番");
  });

  it("去除罗马数字尾部（II / III），需空格分隔", () => {
    expect(normalizeName("某番 II")).toBe("某番");
    expect(normalizeName("某番 III")).toBe("某番");
    // 紧贴字母的 ii 视为标题一部分，不剥
    expect(normalizeName("xxvii")).toBe("xxvii");
  });

  it("去除连续尾数字", () => {
    expect(normalizeName("某番 2")).toBe("某番");
    expect(normalizeName("某番22")).toBe("某番");
  });

  it("纯数字标题保护：不为空则不剥", () => {
    expect(normalizeName("86")).toBe("86");
    expect(normalizeName("3")).toBe("3");
  });

  it("多余分隔符归一为空格", () => {
    expect(normalizeName("hunter-x-hunter")).toBe("hunter x hunter");
    expect(normalizeName("某番——特别篇")).toBe("某番 特别篇");
  });
});

describe("anime-search mergeKey", () => {
  it("纯数字差异不合并：xxx 2 与 xxx 3 key 不同", () => {
    expect(mergeKey("某番 2")).not.toBe(mergeKey("某番 3"));
    expect(mergeKey("某番 2")).not.toBe(mergeKey("某番"));
  });

  it("分隔符数字与裸数字等价：xxx - 2 ≡ xxx 2", () => {
    expect(mergeKey("某番 - 2")).toBe(mergeKey("某番 2"));
    expect(mergeKey("某番2")).toBe(mergeKey("某番 2"));
  });

  it("季节词被吸收，可与基础名合并", () => {
    expect(mergeKey("某番 第二季")).toBe(mergeKey("某番"));
    expect(mergeKey("某番 S02")).toBe(mergeKey("某番"));
    expect(mergeKey("某番 2期")).toBe(mergeKey("某番"));
    expect(mergeKey("某番 II")).toBe(mergeKey("某番"));
    expect(mergeKey("某番 第2季")).toBe(mergeKey("某番"));
  });

  it("括号差异可合并", () => {
    expect(mergeKey("名侦探柯南【简体】")).toBe(mergeKey("名侦探柯南"));
    expect(mergeKey("某番 (2024)")).toBe(mergeKey("某番"));
  });

  it("纯数字标题不产生判别子", () => {
    expect(mergeKey("86")).toBe("86");
  });
});

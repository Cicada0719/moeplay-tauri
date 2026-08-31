import { describe, expect, it } from "vitest";
import { rankPaletteEntries, scorePaletteMatch } from "./matcher";

const entry = (label: string, aliases: string[] = []) => ({ label, aliases, id: label });

describe("palette matcher", () => {
  it("scores exact/prefix/substring on the primary label", () => {
    expect(scorePaletteMatch("游戏库", "游戏库")).toBe(100);
    expect(scorePaletteMatch("游", "游戏库")).toBe(90);
    expect(scorePaletteMatch("戏库", "游戏库")).toBe(79);
    expect(scorePaletteMatch("zzz", "游戏库")).toBe(0);
    expect(scorePaletteMatch("", "游戏库")).toBe(0);
  });

  it("matches aliases with a slight precedence under primary", () => {
    expect(scorePaletteMatch("lib", "游戏库", ["Library", "GameLib"])).toBe(88);
    expect(scorePaletteMatch("gameli", "游戏库", ["Library", "GameLib"])).toBe(88);
    expect(scorePaletteMatch("libraryx", "游戏库", ["Library"])).toBe(0);
  });

  it("matches pinyin full and initials", () => {
    expect(scorePaletteMatch("youxiku", "游戏库")).toBeGreaterThan(0);
    expect(scorePaletteMatch("yxk", "游戏库")).toBeGreaterThan(0);
    expect(scorePaletteMatch("awodk", "游戏库")).toBe(0);
  });

  it("ranks entries by score and drops non-matches", () => {
    const entries = [entry("游戏库"), entry("番剧"), entry("漫画"), entry("小说")];
    const ranked = rankPaletteEntries("游", entries);
    expect(ranked.map((r) => r.entry.label)).toEqual(["游戏库"]);
    const all = rankPaletteEntries("游戏", [entry("游戏"), entry("游戏计划"), entry("其他")]);
    expect(all[0].entry.label).toBe("游戏");
    expect(all[1].entry.label).toBe("游戏计划");
    expect(rankPaletteEntries("zz", entries)).toEqual([]);
    expect(rankPaletteEntries("", entries)).toEqual([]);
  });
});

import { describe, expect, it } from "vitest";
import {
  buildNovelItems,
  buildHandheldChannels,
  buildHandheldGameSystems,
  buildSections,
  buildHandheldQuickNavGroups,
  groupGamesBySystem,
  mergeAnimeItems,
  mergeComicItems,
  migrateHandheldMemory,
  normalizePlatform,
  platformLabel,
  resolveEmulatorAssignments,
  sortRecentItems,
  wrappedIndex,
} from "./systems";

describe("handheld systems grouping", () => {
  it("normalizes emulator engine strings and casing", () => {
    expect(normalizePlatform("Emulator: psp")).toBe("psp");
    expect(normalizePlatform("PSP")).toBe("psp");
    expect(normalizePlatform("  ")).toBe("other");
    expect(normalizePlatform(null)).toBe("other");
  });

  it("groups visible games by platform, sorted by count", () => {
    const games = [
      { id: "1", name: "B", game_type: "gba" },
      { id: "2", name: "A", game_type: "Emulator: psp" },
      { id: "3", name: "C", game_type: "gba" },
      { id: "4", name: "D", game_type: "gba", hidden: true },
      { id: "5", name: "E", game_type: "psp" },
    ];
    const systems = groupGamesBySystem(games);
    expect(systems.map((s) => s.id)).toEqual(["gba", "psp"]);
    expect(systems[0].games.map((g) => g.name)).toEqual(["B", "C"]);
    expect(systems[0].label).toBe("GBA");
    expect(systems[1].label).toBe("PSP");
  });

  it("sorts favorites first within a system", () => {
    const systems = groupGamesBySystem([
      { id: "1", name: "Zelda", game_type: "gba" },
      { id: "2", name: "Advance Wars", game_type: "gba", favorite: true },
    ]);
    expect(systems[0].games.map((g) => g.name)).toEqual(["Advance Wars", "Zelda"]);
  });

  it("falls back to uppercase for unknown platform labels", () => {
    expect(platformLabel("wonderswan")).toBe("WONDERSWAN");
    expect(platformLabel("psp")).toBe("PSP");
  });
});

describe("handheld emulator assignment", () => {
  const installed = [
    { packageName: "com.retroarch.aarch64", label: "RetroArch" },
    { packageName: "org.ppsspp.ppsspp", label: "PPSSPP" },
    { packageName: "com.dsemu.drastic", label: "DraStic" },
  ];

  it("prefers dedicated emulators over RetroArch fallback", () => {
    const result = resolveEmulatorAssignments(["psp", "nds", "gba"], installed);
    expect(result.assignments["psp"]).toBe("org.ppsspp.ppsspp");
    expect(result.assignments["nds"]).toBe("com.dsemu.drastic");
    // gba 无专用模拟器（未装 Pizza Boy / My Boy），回退 RetroArch
    expect(result.assignments["gba"]).toBe("com.retroarch.aarch64");
    expect(result.fallback).toContain("gba");
    expect(result.fallback).not.toContain("psp");
    expect(result.missing).toEqual([]);
    expect(result.labels["psp"]).toBe("PPSSPP");
  });

  it("marks platforms missing when no emulator installed at all", () => {
    const result = resolveEmulatorAssignments(["psp"], []);
    expect(result.missing).toEqual(["psp"]);
    expect(result.assignments["psp"]).toBeUndefined();
  });

  it("uses first installed emulator when RetroArch absent", () => {
    const result = resolveEmulatorAssignments(["nes"], [
      { packageName: "org.ppsspp.ppsspp", label: "PPSSPP" },
    ]);
    expect(result.assignments["nes"]).toBe("org.ppsspp.ppsspp");
    expect(result.fallback).toContain("nes");
  });
});

describe("handheld full-content sections", () => {
  it("keeps the first-level handheld channels stable, including empty channels", () => {
    const channels = buildHandheldChannels({ recentCount: 0, gameCount: 3, animeCount: 0, comicCount: 2, novelCount: 0 });
    expect(channels.map((channel) => channel.id)).toEqual(["recent", "games", "anime", "comic", "novel"]);
    expect(channels.map((channel) => channel.count)).toEqual([0, 3, 0, 2, 0]);
  });

  it("builds the all-games option followed by populated emulator systems", () => {
    expect(buildHandheldGameSystems([
      { id: "psp", label: "PSP", count: 4 },
      { id: "empty", label: "Empty", count: 0 },
      { id: "gba", label: "GBA", count: 2 },
    ])).toEqual([
      { id: "all", label: "全部游戏", count: 6 },
      { id: "psp", label: "PSP", count: 4 },
      { id: "gba", label: "GBA", count: 2 },
    ]);
  });

  it("wraps channel and platform selection without producing invalid indices", () => {
    expect(wrappedIndex(0, -1, 5)).toBe(4);
    expect(wrappedIndex(4, 1, 5)).toBe(0);
    expect(wrappedIndex(0, 1, 0)).toBe(0);
  });

  it("migrates legacy platform and item memory into the two-level model", () => {
    expect(migrateHandheldMemory({ section: "psp", key: "game-7" }, ["psp", "gba"])).toEqual({
      version: 3,
      channel: "games",
      gameSystemId: "psp",
      focusByView: {},
      keyByView: { "games:psp": "game-7" },
    });
    expect(migrateHandheldMemory({ section: "comic", key: "comic-1" }, ["psp"])).toEqual({
      version: 3,
      channel: "comic",
      gameSystemId: "all",
      focusByView: {},
      keyByView: { comic: "comic-1" },
    });
  });

  it("merges anime history with collection, dedup by name, time desc", () => {
    const items = mergeAnimeItems(
      [
        { key: "k1", name: "葬送的芙莉莲", image: "a.jpg", lastEpisodeName: "第 8 集", updatedAt: "2026-08-28T10:00:00Z" },
        { key: "k2", name: "老友记", image: "b.jpg", lastEpisodeName: "第 1 集", updatedAt: "2026-08-20T10:00:00Z" },
      ],
      [
        { name: "葬送的芙莉莲", image: "a.jpg", collectType: 1, updatedAt: "2026-08-29T00:00:00Z" },
        { name: "星际牛仔", image: "c.jpg", collectType: 2, updatedAt: "2026-08-25T00:00:00Z" },
      ],
    );
    expect(items.map((i) => i.title)).toEqual(["葬送的芙莉莲", "星际牛仔", "老友记"]);
    expect(items[0].fromHistory).toBe(true);
    expect(items[1].fromHistory).toBe(false);
    expect(items[1].subtitle).toBe("想看");
    expect(items[0].subtitle).toBe("第 8 集");
  });

  it("merges comic history with favorites, history first", () => {
    const items = mergeComicItems(
      [{ id: "m1", title: "航海王", thumb_url: "t.jpg", last_title: "第 1100 话", ts: 1756400000000 }],
      [
        { id: "m1", title: "航海王", thumb_url: "t.jpg" },
        { id: "m2", title: "进击的巨人", thumb_url: "g.jpg", author: "谏山创" },
      ],
    );
    expect(items.map((i) => i.key)).toEqual(["m1", "m2"]);
    expect(items[0].subtitle).toBe("第 1100 话");
    expect(items[1].subtitle).toBe("谏山创");
    expect(items[1].fromHistory).toBe(false);
  });

  it("builds novel items with chapter + progress subtitle", () => {
    const items = buildNovelItems([
      { key: "n1", title: "诡秘之主", chapterTitle: "第 100 章", progress: 0.42, updatedAt: 1756400000000 },
      { key: "n2", title: "三体", chapterTitle: null, progress: 0, updatedAt: 1756300000000 },
    ]);
    expect(items[0].subtitle).toBe("第 100 章 · 42%");
    expect(items[1].subtitle).toBe("");
    expect(items[0].cover).toBe("");
  });

  it("sorts recent items by time desc and applies limit", () => {
    const items = sortRecentItems(
      [
        { kind: "game", key: "g1", title: "老滚", cover: "", subtitle: "", time: 100, fromHistory: true },
        { kind: "anime", key: "a1", title: "番 A", cover: "", subtitle: "", time: 300, fromHistory: true },
        { kind: "comic", key: "c1", title: "漫 B", cover: "", subtitle: "", time: 200, fromHistory: true },
      ],
      2,
    );
    expect(items.map((i) => i.kind)).toEqual(["anime", "comic"]);
  });

  it("builds section order: recent → media trio → game systems; recent hidden when empty", () => {
    const withRecent = buildSections({
      recentCount: 3,
      animeCount: 2,
      comicCount: 0,
      novelCount: 0,
      gameSystems: [
        { id: "psp", label: "PSP", count: 40 },
        { id: "gba", label: "GBA", count: 12 },
      ],
    });
    expect(withRecent.map((s) => s.id)).toEqual(["recent", "games", "anime", "comic", "novel", "psp", "gba"]);
    expect(withRecent[0].kind).toBe("recent");
    expect(withRecent[1].kind).toBe("games");
    expect(withRecent[2].kind).toBe("media");
    expect(withRecent[5].kind).toBe("games");

    const withoutRecent = buildSections({
      recentCount: 0,
      animeCount: 0,
      comicCount: 0,
      novelCount: 0,
      gameSystems: [],
    });
    expect(withoutRecent.map((s) => s.id)).toEqual(["anime", "comic", "novel"]);
  });
});

describe("handheld quick navigation", () => {
  it("exposes every user-facing content, tool, import, and system route once", () => {
    const groups = buildHandheldQuickNavGroups();
    const views = groups.flatMap((group) => group.items.map((item) => item.view));

    expect(groups.map((group) => group.id)).toEqual(["content", "tools", "system"]);
    expect(new Set(views).size).toBe(views.length);
    expect(views).toEqual(expect.arrayContaining([
      "home", "game-library", "records", "anime", "comic", "novel",
      "continue", "discovery", "scraper", "tasks", "sources", "downloads",
      "backup", "stats", "handheld-import", "steam-import", "emulator",
      "diagnostics", "settings",
    ]));
    expect(views).not.toContain("__tools");
    expect(views).not.toContain("__bigpicture");
    expect(views).not.toContain("handheld");
  });
});

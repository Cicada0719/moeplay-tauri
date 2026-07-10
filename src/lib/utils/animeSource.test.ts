import { describe, expect, it } from "vitest";
import { extractEpisodeNumber, findBestEpisodeMatch, normalizeAnimeTitle, rankSearchItems } from "./animeSource";

describe("anime source matching", () => {
  it("removes common source-site title noise", () => {
    expect(normalizeAnimeTitle("【高清】葬送的芙莉莲 - 在线观看")).toBe("葬送的芙莉莲");
  });
  it("ranks the exact title ahead of spin-offs", () => {
    expect(rankSearchItems("日常", [
      { name: "坂本日常", url: "spin" }, { name: "日常（全集）", url: "exact" }, { name: "日常 解说", url: "commentary" },
    ]).map((item) => item.url)).toEqual(["exact", "spin", "commentary"]);
  });
  it("parses common episode labels", () => {
    expect(extractEpisodeNumber("第十二话")).toBe(12);
    expect(extractEpisodeNumber("S02E03 1080P")).toBe(3);
    expect(extractEpisodeNumber("EP 07")).toBe(7);
  });
  it("matches across all roads instead of trusting array index", () => {
    const roads = [
      { name: "A", episodes: [{ name: "第1集", url: "a1" }] },
      { name: "B", episodes: [{ name: "第11集", url: "b11" }, { name: "第12集", url: "b12" }] },
    ];
    expect(findBestEpisodeMatch(roads, { episodeName: "第12话", episodeIndex: 11 })?.episode.url).toBe("b12");
  });
  it("refuses to silently play episode one when a numbered episode is missing", () => {
    expect(findBestEpisodeMatch([{ name: "A", episodes: [{ name: "第1集", url: "wrong" }] }], { episodeName: "第12集", episodeIndex: 11 })).toBeNull();
  });
});

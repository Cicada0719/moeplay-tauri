import { describe, expect, it } from "vitest";
import {
  DEFAULT_DISPLAY_LIMIT,
  DEFAULT_PER_SOURCE_LIMIT,
  formatSourceBadge,
  mergeSearchResults,
  type SearchItemLike,
} from "./merge";

function item(name: string, url: string): SearchItemLike {
  return { name, url };
}

describe("anime-search mergeSearchResults", () => {
  it("跨源同名合并为一条且 sources 累计、items 与 sources 一一对应", () => {
    const { entries, total } = mergeSearchResults(
      [
        ["源A", [item("进击的巨人", "a/1")]],
        ["源B", [item("进击的巨人【高清】", "b/1")]],
        ["源C", [item("进击的巨人 第二季", "c/1")]],
      ],
      "进击的巨人",
    );
    expect(total).toBe(1);
    expect(entries).toHaveLength(1);
    expect(entries[0].name).toBe("进击的巨人");
    expect(entries[0].sources).toEqual(["源A", "源B", "源C"]);
    expect(entries[0].items).toEqual([item("进击的巨人", "a/1"), item("进击的巨人【高清】", "b/1"), item("进击的巨人 第二季", "c/1")]);
  });

  it("纯数字差异不合并", () => {
    const { entries } = mergeSearchResults(
      [
        ["源A", [item("某番 2", "a/2")]],
        ["源B", [item("某番 3", "b/3")]],
      ],
      "某番",
    );
    expect(entries).toHaveLength(2);
  });

  it("排序：含关键词子串优先 > sources 数降序 > 到达序", () => {
    const { entries } = mergeSearchResults(
      [
        ["源A", [item("无关作品", "a/1"), item("巨人中学", "a/2")]],
        ["源B", [item("无关作品", "b/1"), item("进击的巨人", "b/2")]],
        ["源C", [item("无关作品", "c/1")]],
      ],
      "进击",
    );
    // "进击的巨人" 含关键词 → 第一；"巨人中学" 含"巨人"但不含"进击" → 与"无关作品"同档，
    // "无关作品" 3 源 > "巨人中学" 1 源。
    expect(entries.map((e) => e.name)).toEqual(["进击的巨人", "无关作品", "巨人中学"]);
    expect(entries[1].sources).toEqual(["源A", "源B", "源C"]);
  });

  it("每源截 top N（默认 10），超出部分不参与合并", () => {
    const many = Array.from({ length: 15 }, (_, i) => item(`番剧${i + 1}`, `a/${i + 1}`));
    const { entries, total } = mergeSearchResults([["源A", many]], "番剧", {
      limit: Number.POSITIVE_INFINITY,
    });
    expect(total).toBe(DEFAULT_PER_SOURCE_LIMIT);
    expect(entries).toHaveLength(10);
    expect(entries.some((e) => e.name === "番剧11")).toBe(false);
  });

  it("默认总截断 top 24，total 保留截断前数量", () => {
    const grouped: [string, SearchItemLike[]][] = ["源A", "源B", "源C"].map((s, si) => [
      s,
      Array.from({ length: 10 }, (_, i) => item(`番剧${si * 10 + i + 1}`, `${s}/${i}`)),
    ]);
    const { entries, total } = mergeSearchResults(grouped, "番剧");
    expect(total).toBe(30);
    expect(entries).toHaveLength(DEFAULT_DISPLAY_LIMIT);
    expect(DEFAULT_DISPLAY_LIMIT).toBe(24);
  });

  it("同源自重名只保留第一条", () => {
    const { entries } = mergeSearchResults(
      [["源A", [item("某番", "a/1"), item("某番【重发】", "a/2")]]],
      "某番",
    );
    expect(entries).toHaveLength(1);
    expect(entries[0].items).toEqual([item("某番", "a/1")]);
    expect(entries[0].sources).toEqual(["源A"]);
  });

  it("空输入与非法条目安全跳过", () => {
    const { entries, total } = mergeSearchResults(
      [
        ["", [item("某番", "a/1")]],
        ["源A", [item("", "a/2"), item("某番", "")]],
      ],
      "某番",
    );
    expect(total).toBe(0);
    expect(entries).toHaveLength(0);
  });
});

describe("anime-search formatSourceBadge", () => {
  it("单源直接显示源名", () => {
    expect(formatSourceBadge(["源A"])).toBe("源A");
  });
  it("双源用 · 连接", () => {
    expect(formatSourceBadge(["源A", "源B"])).toBe("源A · 源B");
  });
  it("三源以上折叠为 首两源 +N", () => {
    expect(formatSourceBadge(["源A", "源B", "源C", "源D"])).toBe("源A · 源B +2");
    expect(formatSourceBadge(["源A", "源B", "源C"])).toBe("源A · 源B +1");
  });
});

import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

function source(path: string): string {
  return readFileSync(resolve(process.cwd(), path), "utf8");
}

describe("anime search merged-grid contract", () => {
  it("AnimePage 搜索区使用合并网格 + MediaCard 传 imageSrc + 来源徽标", () => {
    const page = source("src/lib/components/AnimePage.svelte");
    expect(page).toContain('class="search-grid"');
    expect(page).toContain("visibleMergedResults");
    expect(page).toContain("imageSrc={cover || undefined}");
    expect(page).toContain("subtitle={formatSourceBadge(entry.sources)}");
    expect(page).toContain("openResult(entry.sources[0], entry.items[0]");
    expect(page).toContain("显示更多");
    expect(page).toContain("ensureSearchCovers");
  });

  it("封面渐显仅 opacity 动画且 reduced-motion 双写降级", () => {
    const page = source("src/lib/components/AnimePage.svelte");
    expect(page).toContain("@keyframes search-cover-fade");
    expect(page).toContain("prefers-reduced-motion: reduce");
    expect(page).toContain('data-motion="reduce"');
    // 渐显动画只允许 opacity（从 from/to 块中不出现 transform 侧写）
    const fade = page.slice(page.indexOf("@keyframes search-cover-fade"));
    expect(fade.slice(0, fade.indexOf("}"))).not.toContain("transform");
  });

  it("store 暴露合并结果与封面补全管线", () => {
    const store = source("src/lib/stores/anime.svelte.ts");
    expect(store).toContain("mergeSearchResults");
    expect(store).toContain("createSearchCoverFetcher");
    expect(store).toContain("_refreshMergedSearch");
    expect(store).toContain("ensureSearchCovers");
    expect(store).toContain("getSearchCover");
    expect(store).toContain("mergedSearchResults");
  });

  it("anime-search 模块提供 normalize/merge/covers 纯函数实现", () => {
    const normalize = source("src/lib/features/anime-search/normalize.ts");
    const merge = source("src/lib/features/anime-search/merge.ts");
    const covers = source("src/lib/features/anime-search/covers.ts");
    expect(normalize).toContain("export function normalizeName");
    expect(normalize).toContain("export function mergeKey");
    expect(merge).toContain("export function mergeSearchResults");
    expect(merge).toContain("export function formatSourceBadge");
    expect(covers).toContain("export function createSearchCoverFetcher");
  });
});

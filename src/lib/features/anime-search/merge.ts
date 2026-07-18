// 跨源搜索结果合并去重
//
// 输入 [source, items][]（流式到达顺序），输出按相关度排序的合并列表：
//   1. 每源截 top N（默认 10），同名（mergeKey 相同）聚合为一条；
//   2. 排序键：name 含原始关键词子串优先 → sources.length 降序 → 首个到达序升序；
//   3. 默认输出 top 24（DEFAULT_DISPLAY_LIMIT），调用方可传 limit 覆盖（"显示更多"）。

import { mergeKey } from "./normalize";

export interface SearchItemLike {
  name: string;
  url: string;
}

export interface MergedSearchEntry {
  key: string; // mergeKey(item.name)
  name: string; // 首个到达 item 的展示名
  sources: string[]; // 与 items 一一对应（每源取第一条）
  items: SearchItemLike[];
}

export interface MergeOptions {
  perSourceLimit?: number;
  limit?: number;
}

export interface MergeResult {
  entries: MergedSearchEntry[]; // 已按 limit 截断
  total: number; // 截断前的合并总数
}

export const DEFAULT_PER_SOURCE_LIMIT = 10;
export const DEFAULT_DISPLAY_LIMIT = 24;

export function mergeSearchResults(
  grouped: [string, SearchItemLike[]][],
  keyword: string,
  options: MergeOptions = {},
): MergeResult {
  const perSource = options.perSourceLimit ?? DEFAULT_PER_SOURCE_LIMIT;
  const limit = options.limit ?? DEFAULT_DISPLAY_LIMIT;
  const groups = new Map<string, MergedSearchEntry & { seq: number }>();
  let seq = 0;

  for (const [source, items] of grouped) {
    if (!source || !Array.isArray(items)) continue;
    const seenInSource = new Set<string>();
    for (const item of items.slice(0, perSource)) {
      if (!item?.name || !item.url) continue;
      const key = mergeKey(item.name);
      if (!key || seenInSource.has(key)) continue; // 同源自重名只留第一条
      seenInSource.add(key);
      const existing = groups.get(key);
      if (existing) {
        existing.sources.push(source);
        existing.items.push({ name: item.name, url: item.url });
      } else {
        groups.set(key, {
          key,
          name: item.name.trim(),
          sources: [source],
          items: [{ name: item.name, url: item.url }],
          seq: seq++,
        });
      }
    }
  }

  const kw = keyword.trim().toLowerCase();
  const sorted = [...groups.values()].sort((a, b) => {
    const ac = kw && a.name.toLowerCase().includes(kw) ? 0 : 1;
    const bc = kw && b.name.toLowerCase().includes(kw) ? 0 : 1;
    if (ac !== bc) return ac - bc;
    if (a.sources.length !== b.sources.length) return b.sources.length - a.sources.length;
    return a.seq - b.seq;
  });

  return {
    entries: sorted.slice(0, limit).map(({ seq: _seq, ...entry }) => entry),
    total: sorted.length,
  };
}

/** 来源徽标文案：1 源→源名；2 源→"A · B"；更多→"A · B +N"。 */
export function formatSourceBadge(sources: string[]): string {
  if (sources.length <= 2) return sources.join(" · ");
  return `${sources[0]} · ${sources[1]} +${sources.length - 2}`;
}

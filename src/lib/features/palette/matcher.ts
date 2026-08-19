// 命令面板匹配器：普通子串 + 拼音全拼/首字母（与游戏库搜索同源，pinyin-pro）
import { pinyin } from "pinyin-pro";

export interface PaletteEntry {
  id: string;
  label: string;
  hint?: string;
  aliases?: readonly string[];
  group?: string;
  run: () => void | Promise<void>;
}

export interface ScoredPaletteEntry<T> {
  entry: T;
  score: number;
}

/** 归一化小写并去空格（pinyin-pro string 模式下拼音以空格分隔）。 */
function normalized(value: string): string {
  return value.trim().toLowerCase().replace(/\s+/g, "");
}

/** 越大越靠前；0 表示不匹配。 */
export function scorePaletteMatch(query: string, label: string, aliases: readonly string[] = []): number {
  const q = normalized(query);
  if (!q) return 0;
  const primary = normalized(label);
  if (primary === q) return 100;
  if (primary.startsWith(q)) return 90;
  const primaryIndex = primary.indexOf(q);
  if (primaryIndex >= 0) return 80 - Math.min(primaryIndex, 20);
  for (const alias of aliases) {
    const a = normalized(alias);
    if (!a) continue;
    if (a === q) return 98;
    if (a.startsWith(q)) return 88;
    const aliasIndex = a.indexOf(q);
    if (aliasIndex >= 0) return 78 - Math.min(aliasIndex, 20);
  }
  try {
    const full = normalized(pinyin(label, { toneType: "none", type: "string" }));
    const initials = normalized(pinyin(label, { pattern: "first", toneType: "none", type: "string" }));
    if (full === q) return 76;
    if (full.startsWith(q)) return 72;
    if (initials === q) return 70;
    if (initials.startsWith(q)) return 68;
    const fullIndex = full.indexOf(q);
    if (fullIndex >= 0) return 60 - Math.min(fullIndex, 12);
    const initIndex = initials.indexOf(q);
    if (initIndex >= 0) return 52 - Math.min(initIndex, 12);
  } catch {
    // 非 CJK 或 pinyin-pro 异常时忽略拼音匹配
  }
  return 0;
}

export function rankPaletteEntries<T extends { label: string; aliases?: readonly string[] }>(
  query: string,
  entries: readonly T[],
): ScoredPaletteEntry<T>[] {
  const scored: ScoredPaletteEntry<T>[] = [];
  const q = normalized(query);
  if (!q) return scored;
  for (const entry of entries) {
    const score = scorePaletteMatch(q, entry.label, entry.aliases ?? []);
    if (score > 0) scored.push({ entry, score });
  }
  return scored.sort((a, b) => b.score - a.score || a.entry.label.localeCompare(b.entry.label, "zh-CN"));
}

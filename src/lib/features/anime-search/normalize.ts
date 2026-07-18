// 番剧名归一化（跨源搜索结果合并去重用）
//
// 原则：宁可不合并也不错并。
// - normalizeName：激进归一（去括号/季度词/罗马尾数字/尾数字），用于展示与封面查询；
// - mergeKey：在归一化基础上给"纯尾数字差异"追加判别子，
//   使 "xxx 2" 与 "xxx 3" 不会被错误合并，而 "xxx 第二季" 可与 "xxx" 合并。

const BRACKET_PAIR = /[\[【(（][^\]】)）]*[\]】)）]/g;
const SEPARATORS = /[\-–—_·:：,，.。/|~!！?？]+/g;
const WHITESPACE = /\s+/g;
const CN_NUM = "0-9０-９一二三四五六七八九十百零两";
const DIGIT = "0-9０-９";

const SEASON_SUFFIX_RES: RegExp[] = [
  new RegExp(`\\s*第[${CN_NUM}]+[季期部]$`, "u"), // 第二季 / 第3期 / 第10部
  new RegExp(`\\s*[${DIGIT}]+期$`, "u"), // 2期
  /\s*season\s?\d{1,3}$/u, // Season 2
  /\s*s\d{1,3}$/u, // S02 / s2（s 与数字必须相邻，避免误伤 "...is 2"）
  /\s+(?:viii|vii|iii|iv|ii|vi|ix|v)$/u, // 罗马数字尾部（须空格分隔，排除单字母 i/x）
];
const TRAILING_DIGITS_RE = new RegExp(`\\s*[${DIGIT}]+$`, "u");
const TRAILING_DIGITS_MATCH = new RegExp(`([${DIGIT}]+)$`, "u");

/** 替换命中片段；若结果为空则保留原串（保护 "86"、"3" 这类纯数字标题）。 */
function stripIfNonEmpty(s: string, re: RegExp): string {
  const next = s.replace(re, "").trim();
  return next ? next : s;
}

/** 阶段一：小写化 → 去括号及内容 → 分隔符归一 → 空格压缩。 */
function stage1(raw: string): string {
  let s = raw.toLowerCase().trim();
  // 循环处理简单嵌套括号，如 "a（b[c]）"
  for (let i = 0; i < 3; i++) {
    const next = s.replace(BRACKET_PAIR, " ");
    if (next === s) break;
    s = next;
  }
  return s.replace(SEPARATORS, " ").replace(WHITESPACE, " ").trim();
}

/** 阶段二：循环剥离尾部季度/期数/罗马标记；includeDigits 时连同裸尾数字一起剥离。 */
function stripSuffixes(s: string, includeDigits: boolean): string {
  let out = s;
  for (let i = 0; i < 4; i++) {
    const before = out;
    for (const re of SEASON_SUFFIX_RES) out = stripIfNonEmpty(out, re);
    if (includeDigits) out = stripIfNonEmpty(out, TRAILING_DIGITS_RE);
    if (out === before) break;
  }
  return out;
}

/**
 * 展示用归一化：去括号、去季度/集数后缀（第二季 / S02 / 2期 / - 2 / II / III / 连续尾数字）、
 * 分隔符与空格压缩。纯数字标题（"86"）保持原样。
 */
export function normalizeName(raw: string): string {
  return stripSuffixes(stage1(raw), true);
}

/**
 * 合并用 key：在 normalizeName 之上，为"被剥离的裸尾数字"追加 `#数字` 判别子。
 * 季节词（第二季/S02/2期/II）整体被吸收，不产生判别子 → 可与基础名合并；
 * 裸尾数字（"xxx 2"、"xxx - 2"、"xxx2"）保留判别子 → "xxx 2" 与 "xxx 3" 不合并。
 */
export function mergeKey(raw: string): string {
  const base = normalizeName(raw);
  // 先只剥季节词（保留尾数字），再取剩余尾数字作为判别子
  const keepDigits = stripSuffixes(stage1(raw), false);
  const m = keepDigits.match(TRAILING_DIGITS_MATCH);
  if (!m) return base;
  const stem = keepDigits.slice(0, keepDigits.length - m[1].length).trim();
  if (!stem) return base; // 纯数字标题，不做判别
  return `${base}#${m[1]}`;
}

// 萌游 MoeGame · 规则引擎 API 封装（对应 spec §3.1/3.2/3.4）
//
// 与 Rust 侧 camelCase serde 输出一一对应；统一走 invokeCmd 以便测试注入 mock。

import { invokeCmd } from "./core";

// ── 类型（与 Rust schema.rs / engine.rs 对齐）────────────────────────────

export type ContentType = "anime" | "manga" | "novel";
export type RuleOrigin = "builtin" | "custom";
export type RuleStatus = "ready" | "invalid" | "loading";

export interface RuleManifest {
  name: string;
  version: string;
  contentType: ContentType;
  baseUrl: string;
  language: string;
  nsfw: boolean;
  author?: string | null;
  search: string;
  detail: string;
  chapter: string;
  parse: string;
}

export interface RuleLoadError {
  message: string;
  line?: number | null;
  phase: string;
}

export interface LoadedRule {
  id: string;
  manifest: RuleManifest;
  origin: RuleOrigin;
  status: RuleStatus;
  error?: RuleLoadError | null;
}

export interface SearchItem {
  title: string;
  url: string;
  cover?: string | null;
  extra?: unknown;
}

export interface Detail {
  title: string;
  cover?: string | null;
  description?: string | null;
  extra?: unknown;
}

export interface Chapter {
  id: string;
  title: string;
  url: string;
  index: number;
}

export interface ParseResult {
  urls: string[];
  kind: string;
  headers?: Record<string, string> | null;
}

/** RuleExecError 的 kind tag（serde tag = "kind", camelCase） */
export type RuleExecErrorKind =
  | "ruleNotFound"
  | "timeout"
  | "cancelled"
  | "scriptError"
  | "network"
  | "badReturn";

export interface RuleExecError {
  kind: RuleExecErrorKind;
  message?: string;
  line?: number | null;
}

// ── 命令封装 ─────────────────────────────────────────────────────────────

export function loadAllRules(): Promise<LoadedRule[]> {
  return invokeCmd<LoadedRule[]>("rules_load_all");
}

export function search(
  ruleId: string,
  keyword: string,
  page = 1,
): Promise<SearchItem[]> {
  return invokeCmd<SearchItem[]>("rules_search", { ruleId, keyword, page });
}

export function detail(ruleId: string, url: string): Promise<Detail> {
  return invokeCmd<Detail>("rules_detail", { ruleId, url });
}

export function chapters(ruleId: string, detailUrl: string): Promise<Chapter[]> {
  return invokeCmd<Chapter[]>("rules_chapters", { ruleId, detailUrl });
}

export function parse(
  ruleId: string,
  chapterUrl: string,
  scope: string,
): Promise<ParseResult> {
  return invokeCmd<ParseResult>("rules_parse", { ruleId, chapterUrl, scope });
}

export function cancelScope(scope: string): Promise<void> {
  return invokeCmd<void>("rules_cancel_scope", { scope });
}

export function importRule(path: string): Promise<LoadedRule> {
  return invokeCmd<LoadedRule>("rules_import", { path });
}

export function removeCustomRule(ruleId: string): Promise<void> {
  return invokeCmd<void>("rules_remove_custom", { ruleId });
}

export function exportRules(path: string): Promise<number> {
  return invokeCmd<number>("rules_export", { path });
}

/** 判断是否为取消类错误（FR-02 竞态静默丢弃依据） */
export function isCancelledError(err: unknown): boolean {
  const e = err as { kind?: string; message?: string } | null;
  const kind = e?.kind?.toLowerCase();
  const message = String(e?.message ?? err ?? "");
  return (
    kind === "cancelled" ||
    kind === "canceled" ||
    message.includes("已取消") ||
    message.toLowerCase().includes("cancelled")
  );
}

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

// ── 规则列表缓存 ─────────────────────────────────────────────────────────
//
// Kimi K3 复审第 1 项：`rules_load_all` 每次都会重扫规则目录并逐条 `compile_check`
// （每条最多 10s），不能在任何一次点击中都重新调用。这里做模块级缓存：加载命令只在
// 首次访问或显式刷新时触发，换源面板/源列表读取缓存即可。

let cachedRules: LoadedRule[] | null = null;
let rulesLoadPromise: Promise<LoadedRule[]> | null = null;

/** 获取已加载规则（带缓存）。仅在缓存未命中时触发 `rules_load_all`。 */
export function getLoadedRules(): Promise<LoadedRule[]> {
  if (cachedRules) return Promise.resolve(cachedRules);
  if (!rulesLoadPromise) {
    rulesLoadPromise = loadAllRules()
      .then((rules) => {
        cachedRules = rules;
        return rules;
      })
      .catch((err) => {
        // 加载失败不缓存 rejected Promise（Kimi K3 复审第 8 项）：把缓存重置为 null，
        // 下一次调用重新发起加载，避免失败后快速换源面板永远失败无法自愈（只能靠
        // 显式 refresh 恢复）。并发等待方仍收到本次错误，rethrow 保持原语义。
        rulesLoadPromise = null;
        throw err;
      });
  }
  return rulesLoadPromise;
}

/** 显式刷新规则缓存（如导入/删除自定义规则后由调用方触发）。 */
export function refreshLoadedRules(): Promise<LoadedRule[]> {
  cachedRules = null;
  rulesLoadPromise = null;
  return getLoadedRules();
}

export function search(
  ruleId: string,
  keyword: string,
  page = 1,
  invocation = "",
): Promise<SearchItem[]> {
  return invokeCmd<SearchItem[]>("rules_search", { ruleId, keyword, page, invocation });
}

export function detail(ruleId: string, url: string): Promise<Detail> {
  return invokeCmd<Detail>("rules_detail", { ruleId, url });
}

export function chapters(
  ruleId: string,
  detailUrl: string,
  invocation = "",
): Promise<Chapter[]> {
  return invokeCmd<Chapter[]>("rules_chapters", { ruleId, detailUrl, invocation });
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
  // 导入成功即失效规则列表缓存并后台重载（Kimi K3 复审第 7 项）：换源面板
  // `getLoadedRules()` 下次访问时拿到最新列表，无需等用户手动刷新。
  return invokeCmd<LoadedRule>("rules_import", { path }).then((rule) => {
    void refreshLoadedRules().catch(() => {});
    return rule;
  });
}

export function removeCustomRule(ruleId: string): Promise<void> {
  // 删除成功同 importRule：失效缓存并后台重载，避免刚删除的源仍在快速换源面板出现。
  return invokeCmd<void>("rules_remove_custom", { ruleId }).then(() => {
    void refreshLoadedRules().catch(() => {});
  });
}

export function exportRules(path: string): Promise<number> {
  return invokeCmd<number>("rules_export", { path });
}

// ── 规则包热更新 + 源健康检查（spec task-02 §3.4/§3.5）────────────────────

/** 单源健康状态（Rust `HealthStatus` 序列化为 PascalCase）。 */
export type HealthStatus = "Healthy" | "Degraded" | "Abnormal" | "Unknown";

export interface SourceHealthInfo {
  sourceId: string;
  status: HealthStatus;
  consecutiveFailures: number;
  lastCheckedAt: number | null;
  lastLatencyMs: number | null;
  lastError: string | null;
}

/** 单次健康探测结果（`rules_probe_health` 返回）。 */
export interface HealthProbeResult {
  sourceId: string;
  ok: boolean;
  latencyMs: number;
  error?: string | null;
}

/** 规则包来源：内置 / 远端热更新缓存。 */
export type RuleSource = "bundled" | "remoteCache";

export interface RulesMetaInfo {
  packageVersion: string;
  source: RuleSource;
  updatedAt: number;
  lastCheckAt: number | null;
  ruleCount: number;
  remoteBase: string;
}

/** 更新结果状态（Rust `UpdateStatus` tag）。 */
export type UpdateStatus = "updated" | "alreadyLatest" | "fallbackCached";

export interface UpdateOutcome {
  status: UpdateStatus;
  fromVersion: string | null;
  toVersion: string;
  updatedRules: number;
  /** `fallbackCached` 时的回退原因。 */
  reason?: string;
}

/** 规则包元信息（设置页展示版本/来源/更新时间）。 */
export function getRulesMeta(): Promise<RulesMetaInfo> {
  return invokeCmd<RulesMetaInfo>("rules_get_meta");
}

/** 检查并更新规则包；`force=true` 跳过 24h 节流（启动时静默调用 force=false）。 */
export function checkAndUpdateRules(force = false): Promise<UpdateOutcome> {
  return invokeCmd<UpdateOutcome>("rules_check_and_update", { force });
}

/** 立即健康检查；`ids` 为空表示全量探测（并发，单源 10s 超时）。 */
export function probeHealth(ids?: string[]): Promise<HealthProbeResult[]> {
  return invokeCmd<HealthProbeResult[]>("rules_probe_health", {
    sourceIds: ids ?? null,
  });
}

/** 读取持久化的源健康状态（源列表展示；不触发探测）。 */
export function getHealth(): Promise<SourceHealthInfo[]> {
  return invokeCmd<SourceHealthInfo[]>("rules_get_health");
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

/** 从 KazumiRules 官方规则库同步并导入（设置页按钮 + 启动自动调用）。 */
export interface KazumiImportResult {
  imported: number;
  catalogTotal: number;
  synced: number;
  unchanged: number;
  syncFailed: number;
  invalid: number;
  errors: string[];
}

export function importKazumiRules(force = false): Promise<KazumiImportResult> {
  return invokeCmd<KazumiImportResult>("anime_import_kazumi_rules", { force });
}

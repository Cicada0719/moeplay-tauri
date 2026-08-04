// 萌游 MoeGame · 源切换状态保持 store（对应 spec §3.5）
//
// 核心入口 `switchSource` 实现 FR-02：
// 1. 作废前序 invocation（rules_cancel_scope）
// 2. 新源搜索 → 章节列表 → 按 chapterIndex 定位（fallback 到最新一集）
// 3. 解析播放地址，返回结构化 SwitchResult
// 竞态：callSeq 计数器 + 独立 invocation scope 双保险，保证并发调用仅最后一次生效
// （前序结果静默丢弃）。

import { writable, type Writable } from "svelte/store";
import {
  cancelScope,
  chapters,
  isCancelledError,
  parse,
  search,
  type Chapter,
  type ParseResult,
} from "../api/rules";

export interface SwitchContext {
  /** 条目标识（搜索项 url 归一化后的稳定 id） */
  contentId: string;
  title: string;
  /** 当前集/话/章 1-based */
  chapterIndex: number;
  /** 当前播放进度（秒），漫画/小说为 0 */
  positionSec: number;
}

export type SwitchStatus = "ok" | "fallback" | "failed";

export interface SwitchResult {
  status: SwitchStatus;
  chapters: Chapter[];
  /** 实际定位到的章节；failed 时为 null */
  targetChapter: Chapter | null;
  parseResult?: ParseResult;
  /** 续播秒数（fallback/failed 时为 0） */
  resumeSec: number;
  /** fallback/failed 时的用户提示文案 */
  message?: string;
  /** 该结果是「被更新的调用取代 / 已取消」的静默丢弃结果。spec §5 承诺 failed 必有
   *  message，消费方凭此标记与真实失败区分：不应驱动任何 UI（AnimePlayer 接线用）。 */
  discarded?: boolean;
}

export interface SourceSwitchState {
  switching: boolean;
  /** "play:{contentId}" */
  currentScope: string | null;
  lastError: string | null;
  /** 最近一次已落地的切换结果（ok/fallback/failed 均含结构化数据），供播放器
   *  容器（任务 3 接线）消费；取消/竞态丢弃时为 null。 */
  lastResult: SwitchResult | null;
}

export const sourceSwitchState: Writable<SourceSwitchState> = writable({
  switching: false,
  currentScope: null,
  lastError: null,
  lastResult: null,
});

/** 递增调用序号：用于丢弃被新调用取代的旧结果（Rust 层之外的 JS 双保险） */
let callSeq = 0;

/** 最近一次 switchSource 的 invocation scope（"play:{contentId}:{seq}"）。新调用用它
 *  作废旧调用的 search/chapters/parse（Kimi K3 复审第 7 项）。 */
let activeInvocation: string | null = null;

/**
 * 标题归一化：trim、小写、全角转半角、去标点、压缩空白。
 * 单独导出便于单测（测试 19）。
 */
export function normalizeTitle(s: string): string {
  return s
    .trim()
    .toLowerCase()
    // 全角 → 半角（FF01-FF5E）
    .replace(/[！-～]/g, (ch) =>
      String.fromCharCode(ch.charCodeAt(0) - 0xfee0),
    )
    // 全角空格 → 普通空格
    .replace(/　/g, " ")
    // 去标点（保留字母/数字/空白）
    .replace(/[^\p{L}\p{N}\s]/gu, "")
    .replace(/\s+/g, " ");
}

/** 竞态丢弃文案：前序切换被更新的切换请求取代。spec §5 要求 status='failed' 时必有
 *  message，此处以固定文案让消费方区分「被取代的丢弃结果」与真实失败（Kimi K3 复审第 4 项）。 */
export const SWITCH_SUPERSEDED_MESSAGE = "请求已被更新请求取代";

/** 构造一个全新的 failed 结果（每次返回新对象，避免共享引用被外部修改）。
 *  `discarded=true` 表示该结果是竞态/取消导致的静默丢弃，非真实失败。 */
function emptyFailed(message?: string, discarded = false): SwitchResult {
  return {
    status: "failed",
    chapters: [],
    targetChapter: null,
    resumeSec: 0,
    ...(message ? { message } : {}),
    ...(discarded ? { discarded: true } : {}),
  };
}

/**
 * 把规则引擎错误映射为用户可读文案。
 * - `ruleNotFound`：源被禁用/不存在（§3.6 在 UI 层已置灰，这里兜底处理「被误标无效」
 *   或状态刷新竞态导致对禁用源发起了切换）；
 * - 其它错误透传 `message`。
 */
function describeSwitchError(err: unknown): string {
  const e = err as { kind?: string; message?: string } | null;
  const kind = e?.kind?.toLowerCase();
  if (kind === "rulenotfound") return "该源当前不可用或已被禁用，请选择其他源";
  return err instanceof Error ? err.message : String(err);
}

/**
 * 将 `SwitchResult` 映射为播放器容器可直接消费的载荷（spec Step 10「结果透传至
 * 播放器容器」的稳定契约，任务 3 接线用）：
 * - `ok` / `fallback`：`url` + `headers` + `kind` 交给 `<video>`，`resumeMs` 用于 seek；
 * - `failed`：`url = null`，由播放器错误 UI（任务 3）读取 `message`。
 *
 * 单位契约：`SwitchResult.resumeSec` 是秒（spec §3.5，来自 `SwitchContext.positionSec`），
 * 而播放器容器的 `_pendingSeekMs` / `playDirectVideoSource(…, seekMs)` 是毫秒——这里统一
 * 换算为 `resumeMs`（`resumeSec * 1000`），避免调用方把秒当毫秒传入导致续播点缩水 1000 倍
 * （Kimi K3 复审第 6 项 high）。
 */
export function switchResultToPlayback(result: SwitchResult): {
  url: string | null;
  headers: Record<string, string> | null;
  kind: string | null;
  resumeMs: number;
  chapterIndex: number | null;
  status: SwitchStatus;
  message?: string;
  discarded?: boolean;
} {
  return {
    url: result.parseResult?.urls?.[0] ?? null,
    headers: result.parseResult?.headers ?? null,
    kind: result.parseResult?.kind ?? null,
    resumeMs: result.resumeSec * 1000,
    chapterIndex: result.targetChapter?.index ?? null,
    status: result.status,
    ...(result.message ? { message: result.message } : {}),
    ...(result.discarded ? { discarded: true } : {}),
  };
}

/**
 * 切换源并保持上下文。并发调用时仅最后一次生效。
 * 全程 try/catch，绝不向上抛未捕获异常（防白屏）。
 *
 * 并发契约（Kimi K3 复审第 7 项）：每次调用生成**独立** invocation scope
 * （`play:{contentId}:{seq}`），search/chapters/parse 全程绑定该 scope。scope 按调用唯一 →
 * 旧调用迟到的 parse 不会取消最新调用已注册的 token；新调用通过 `cancelScope(旧 invocation)`
 * 把旧调用整体作废（而非 parse 单独注册共享 play scope）。
 */
export async function switchSource(
  targetRuleId: string,
  ctx: SwitchContext,
): Promise<SwitchResult> {
  const seq = ++callSeq;
  const invocation = `play:${ctx.contentId}:${seq}`;
  const prevInvocation = activeInvocation;
  activeInvocation = invocation;

  sourceSwitchState.set({ switching: true, currentScope: invocation, lastError: null, lastResult: null });

  try {
    // 1. 作废旧 invocation（其 search/chapters/parse 一起取消）。首次调用无前序，跳过。
    //    取消失败静默降级（Kimi K3 复审第 8 项）：取消旧调用的 IPC 异常不应阻断本次
    //    全新切换，也不把旧调用的原始错误透进 lastError——旧调用最终仍会被 callSeq
    //    校验静默丢弃，不需要向用户展示取消链路上的底层错误。
    if (prevInvocation) {
      try {
        await cancelScope(prevInvocation);
      } catch {
        // 静默降级：新切换照常继续。
      }
    }

    // 2. 搜索匹配条目（spec §3.5 第 2 步）。
    //    §3.6 的置灰在 UI 层拦截禁用源（primary guard）；这里仍处理运行期错误路径：
    //    搜索无结果（notFound）与规则被禁用/不存在（RuleNotFound，由 describeSwitchError
    //    转可读文案），两层并存不冲突。（DeepSeek 复审第 2 项）
    const items = await search(targetRuleId, ctx.title, 1, invocation);
    if (items.length === 0) {
      throw Object.assign(new Error("新源未找到该条目，请检查关键词或稍后重试"), {
        kind: "notFound",
      });
    }
    // 标题归一化比较后取首个匹配项；找不到再回退 items[0]（Kimi K3 复审第 3 项：
    // 源返回的条目标题常有全角/空白/标点差异，固定取 items[0] 可能命中无关条目）。
    const normalized = normalizeTitle(ctx.title);
    const matched = items.find((item) => normalizeTitle(item.title) === normalized);
    const detailUrl = (matched ?? items[0]).url;

    // 3. 章节列表
    const chapterList = await chapters(targetRuleId, detailUrl, invocation);
    if (chapterList.length === 0) {
      throw Object.assign(new Error("新源未解析到剧集"), { kind: "noChapters" });
    }

    // 4. 定位章节
    let target = chapterList.find((c) => c.index === ctx.chapterIndex);
    let status: "ok" | "fallback" = "ok";
    let resumeSec = ctx.positionSec;
    let message: string | undefined;
    if (!target) {
      // 取 index 最大者作为「最新一集」
      target = chapterList.reduce((a, b) => (b.index > a.index ? b : a));
      status = "fallback";
      resumeSec = 0;
      message = `当前源暂无第 ${ctx.chapterIndex} 集，已跳转至最新一集`;
    }

    // 5. 解析播放地址（绑定本次 invocation scope，旧调用无法取消）
    const parseResult = await parse(targetRuleId, target.url, invocation);

    // 双保险：已被更新的调用取代则静默丢弃（discarded 标记 + 文案，区别于真实失败）
    if (seq !== callSeq) return emptyFailed(SWITCH_SUPERSEDED_MESSAGE, true);

    const result: SwitchResult = {
      status,
      chapters: chapterList,
      targetChapter: target,
      parseResult,
      resumeSec,
      message,
    };
    sourceSwitchState.set({
      switching: false,
      currentScope: invocation,
      lastError: null,
      lastResult: result,
    });
    return result;
  } catch (err) {
    // 取消 → 静默丢弃（不更新错误状态，lastResult 保持 null）。竞态被取代（seq !==
    // callSeq）与外部取消当前调用（seq === callSeq）都属非失败结果：spec §5 要求
    // failed 必有 message，故以固定文案 + discarded 标记供消费方区分并跳过 UI。
    if (isCancelledError(err)) {
      const discarded = seq !== callSeq ? SWITCH_SUPERSEDED_MESSAGE : "切换已取消";
      if (seq === callSeq) {
        sourceSwitchState.set({
          switching: false,
          currentScope: invocation,
          lastError: null,
          lastResult: null,
        });
      }
      return emptyFailed(discarded, true);
    }

    if (seq !== callSeq) return emptyFailed(SWITCH_SUPERSEDED_MESSAGE, true);

    // 结构化透传：区分「条目未找到/无剧集」与「规则被禁用/不存在」（§3.6 置灰的
    // 运行时兜底），统一转为 lastError + failed 结果，绝不向上抛未捕获异常。
    const lastError = describeSwitchError(err);
    sourceSwitchState.set({
      switching: false,
      currentScope: invocation,
      lastError,
      lastResult: emptyFailed(lastError),
    });
    return emptyFailed(lastError);
  } finally {
    // 仅当自己是当前 invocation 时清理；被新调用取代的旧调用不碰该状态。
    if (activeInvocation === invocation) activeInvocation = null;
  }
}

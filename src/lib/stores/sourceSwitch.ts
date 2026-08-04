// 萌游 MoeGame · 源切换状态保持 store（对应 spec §3.5）
//
// 核心入口 `switchSource` 实现 FR-02：
// 1. 取消前序 scope（rules_cancel_scope）
// 2. 新源搜索 → 章节列表 → 按 chapterIndex 定位（fallback 到最新一集）
// 3. 解析播放地址，返回结构化 SwitchResult
// 竞态：callSeq 计数器保证并发调用仅最后一次生效（前序结果静默丢弃）。

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

/** 构造一个全新的 failed 结果（每次返回新对象，避免共享引用被外部修改）。 */
function emptyFailed(message?: string): SwitchResult {
  return {
    status: "failed",
    chapters: [],
    targetChapter: null,
    resumeSec: 0,
    ...(message ? { message } : {}),
  };
}

/**
 * 将 `SwitchResult` 映射为播放器容器可直接消费的载荷（spec Step 10「结果透传至
 * 播放器容器」的稳定契约，任务 3 接线用）：
 * - `ok` / `fallback`：`url` + `headers` + `kind` 交给 `<video>`，`resumeSec` 用于 seek；
 * - `failed`：`url = null`，由播放器错误 UI（任务 3）读取 `message`。
 */
export function switchResultToPlayback(result: SwitchResult): {
  url: string | null;
  headers: Record<string, string> | null;
  kind: string | null;
  resumeSec: number;
  chapterIndex: number | null;
  status: SwitchStatus;
  message?: string;
} {
  return {
    url: result.parseResult?.urls?.[0] ?? null,
    headers: result.parseResult?.headers ?? null,
    kind: result.parseResult?.kind ?? null,
    resumeSec: result.resumeSec,
    chapterIndex: result.targetChapter?.index ?? null,
    status: result.status,
    ...(result.message ? { message: result.message } : {}),
  };
}

/**
 * 切换源并保持上下文。并发调用时仅最后一次生效。
 * 全程 try/catch，绝不向上抛未捕获异常（防白屏）。
 */
export async function switchSource(
  targetRuleId: string,
  ctx: SwitchContext,
): Promise<SwitchResult> {
  const scope = `play:${ctx.contentId}`;
  const seq = ++callSeq;

  sourceSwitchState.set({ switching: true, currentScope: scope, lastError: null, lastResult: null });

  try {
    // 1. 取消前序任务（FR-02 竞态取消）
    await cancelScope(scope);

    // 2. 搜索匹配条目（标题归一化后取首个）
    const items = await search(targetRuleId, ctx.title, 1);
    if (items.length === 0) {
      throw Object.assign(new Error("新源未找到该条目，请检查关键词或稍后重试"), {
        kind: "notFound",
      });
    }
    const detailUrl = items[0].url;

    // 3. 章节列表
    const chapterList = await chapters(targetRuleId, detailUrl);
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

    // 5. 解析播放地址
    const parseResult = await parse(targetRuleId, target.url, scope);

    // 双保险：已被更新的调用取代则静默丢弃
    if (seq !== callSeq) return emptyFailed();

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
      currentScope: scope,
      lastError: null,
      lastResult: result,
    });
    return result;
  } catch (err) {
    // 取消 → 静默丢弃（不更新错误状态，lastResult 保持 null）
    if (isCancelledError(err)) {
      if (seq === callSeq) {
        sourceSwitchState.set({
          switching: false,
          currentScope: scope,
          lastError: null,
          lastResult: null,
        });
      }
      return emptyFailed();
    }

    if (seq !== callSeq) return emptyFailed();

    const lastError = err instanceof Error ? err.message : String(err);
    sourceSwitchState.set({
      switching: false,
      currentScope: scope,
      lastError,
      lastResult: emptyFailed(lastError),
    });
    return emptyFailed(lastError);
  }
}

// sourceSwitch store 单元测试（对应 spec §6.2 测试 15~19）
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  SWITCH_SUPERSEDED_MESSAGE,
  normalizeTitle,
  sourceSwitchState,
  switchResultToPlayback,
  switchSource,
  type SwitchContext,
} from "./sourceSwitch";

const cancelledError = () =>
  Object.assign(new Error("任务已取消"), { kind: "cancelled" });

// 可取消的 mock 状态：cancelScope 会拒绝所有在途操作（模拟 Rust 侧 token 取消）
const cancel = vi.hoisted(() => {
  let generation = 0;
  let rejectors: Array<() => void> = [];
  return {
    reset() {
      generation = 0;
      rejectors = [];
    },
    current: () => generation,
    cancelScope: () => {
      generation++;
      for (const r of rejectors.splice(0)) r();
    },
    addRejector: (r: () => void) => {
      rejectors.push(r);
      return () => {
        const i = rejectors.indexOf(r);
        if (i >= 0) rejectors.splice(i, 1);
      };
    },
  };
});

const mocks = vi.hoisted(() => ({
  cancelScope: vi.fn(),
  search: vi.fn(),
  chapters: vi.fn(),
  parse: vi.fn(),
}));

vi.mock("../api/rules", () => ({
  cancelScope: mocks.cancelScope,
  search: mocks.search,
  chapters: mocks.chapters,
  parse: mocks.parse,
  isCancelledError: (e: unknown) => {
    const kind = (e as { kind?: string })?.kind?.toLowerCase();
    return kind === "cancelled" || kind === "canceled";
  },
}));

/** 构造一个在途可取消的 async 操作：被取消则 reject Cancelled，否则 resolve。 */
function cancellable<T>(resolveValue: T, delayMs = 10): Promise<T> {
  return new Promise<T>((resolve, reject) => {
    const myGen = cancel.current();
    const done = cancel.addRejector(() => reject(cancelledError()));
    setTimeout(() => {
      done();
      if (myGen === cancel.current()) resolve(resolveValue);
      else reject(cancelledError());
    }, delayMs);
  });
}

const OK_ITEM = { title: "测试番", url: "https://example.com/detail/1" };
const CHAPTERS_5 = [{ id: "5", title: "第 5 集", url: "https://example.com/5", index: 5 }];
const CHAPTERS_3 = [
  { id: "1", title: "第 1 集", url: "https://example.com/1", index: 1 },
  { id: "2", title: "第 2 集", url: "https://example.com/2", index: 2 },
  { id: "3", title: "第 3 集", url: "https://example.com/3", index: 3 },
];
const PARSE_OK = {
  urls: ["https://cdn.example.com/v.m3u8"],
  kind: "video",
  headers: { Referer: "https://example.com" },
};

const CTX: SwitchContext = {
  contentId: "c1",
  title: "测试番",
  chapterIndex: 5,
  positionSec: 750,
};

beforeEach(() => {
  vi.clearAllMocks();
  cancel.reset();
  sourceSwitchState.set({ switching: false, currentScope: null, lastError: null, lastResult: null });
});

describe("switchSource", () => {
  it("switch_ok_resume: 命中目标集时保持进度", async () => {
    mocks.cancelScope.mockResolvedValue(undefined);
    mocks.search.mockResolvedValue([OK_ITEM]);
    mocks.chapters.mockResolvedValue(CHAPTERS_5);
    mocks.parse.mockResolvedValue(PARSE_OK);

    const result = await switchSource("rule-b", CTX);

    // 首次调用无前序 invocation：无需 cancelScope（Kimi K3 复审第 7 项 per-invocation 设计）
    expect(mocks.cancelScope).not.toHaveBeenCalled();
    expect(result.status).toBe("ok");
    expect(result.resumeSec).toBe(750);
    expect(result.targetChapter?.index).toBe(5);
    expect(result.parseResult?.urls[0]).toBe("https://cdn.example.com/v.m3u8");

    // 结果透传至 store：播放器容器可直接消费 lastResult
    let state: { switching: boolean; currentScope: string | null; lastError: string | null; lastResult: typeof result | null } =
      { switching: true, currentScope: null, lastError: null, lastResult: null };
    const unsub = sourceSwitchState.subscribe((s) => Object.assign(state, s));
    unsub();
    expect(state.lastError).toBeNull();
    expect(state.lastResult?.status).toBe("ok");
    expect(state.lastResult?.resumeSec).toBe(750);
    expect(state.lastResult?.targetChapter?.index).toBe(5);
  });

  it("switch_result_to_playback: 把 SwitchResult 映射为播放器容器可消费的载荷", () => {
    const playback = switchResultToPlayback({
      status: "ok",
      chapters: CHAPTERS_5,
      targetChapter: CHAPTERS_5[0],
      parseResult: PARSE_OK,
      resumeSec: 750,
    });
    expect(playback.url).toBe("https://cdn.example.com/v.m3u8");
    expect(playback.kind).toBe("video");
    expect(playback.headers).toEqual({ Referer: "https://example.com" });
    // 单位契约：resumeSec（秒）→ resumeMs（毫秒），播放器 _pendingSeekMs 按毫秒消费
    expect(playback.resumeMs).toBe(750_000);
    expect(playback.chapterIndex).toBe(5);
    expect(playback.status).toBe("ok");
    expect(playback.message).toBeUndefined();

    // failed：url 为空，错误文案走 message
    const failed = switchResultToPlayback({
      status: "failed",
      chapters: [],
      targetChapter: null,
      resumeSec: 0,
      message: "新源未找到该条目",
    });
    expect(failed.url).toBeNull();
    expect(failed.message).toBe("新源未找到该条目");
    expect(failed.chapterIndex).toBeNull();
    expect(failed.resumeMs).toBe(0);
  });

  it("switch_result_to_playback_resume_unit_is_ms: 续播秒数换算为毫秒（Kimi K3 复审第 6 项 high）", () => {
    // spec §3.5 resumeSec 是秒；播放器容器 playDirectVideoSource(…, seekMs) 与
    // _pendingSeekMs 是毫秒。switchResultToPlayback 必须在边界处 ×1000，否则 750s
    // 的续播点会落成 0.75s（currentTime = _pendingSeekMs / 1000）。
    const cases: Array<[number, number]> = [
      [0, 0],
      [5, 5_000],
      [65, 65_000],
      [750, 750_000],
    ];
    for (const [sec, ms] of cases) {
      const playback = switchResultToPlayback({
        status: "ok",
        chapters: CHAPTERS_5,
        targetChapter: CHAPTERS_5[0],
        parseResult: PARSE_OK,
        resumeSec: sec,
      });
      expect(playback.resumeMs).toBe(ms);
    }
  });

  it("switch_matches_normalized_title: 标题归一化匹配取首个命中项而非固定 items[0]", async () => {
    mocks.cancelScope.mockResolvedValue(undefined);
    // items[0] 是无关条目；命中项在 items[1] 且标题带全角空格/全角标点差异
    mocks.search.mockResolvedValue([
      { title: "别的番剧", url: "https://example.com/detail/other" },
      { title: "　测试番！！", url: "https://example.com/detail/right" },
    ]);
    mocks.chapters.mockResolvedValue(CHAPTERS_5);
    mocks.parse.mockResolvedValue(PARSE_OK);

    const result = await switchSource("rule-b", CTX);

    // 必须取归一化匹配的 detail_url，而非盲目用 items[0]；chapters 绑定本次 invocation scope
    expect(mocks.chapters).toHaveBeenCalledWith(
      "rule-b",
      "https://example.com/detail/right",
      expect.stringMatching(/^play:c1:\d+$/),
    );
    expect(result.status).toBe("ok");
  });

  it("switch_fallback_to_latest: 目标集不存在时跳转最新一集并提示", async () => {
    mocks.cancelScope.mockResolvedValue(undefined);
    mocks.search.mockResolvedValue([OK_ITEM]);
    mocks.chapters.mockResolvedValue(CHAPTERS_3);
    mocks.parse.mockResolvedValue(PARSE_OK);

    const result = await switchSource("rule-b", CTX);

    expect(result.status).toBe("fallback");
    expect(result.targetChapter?.index).toBe(3);
    expect(result.resumeSec).toBe(0);
    expect(result.message).toContain("第 5 集");
    expect(result.message).toContain("最新一集");
  });

  it("switch_race_last_wins: 快速连切仅最后一次生效", async () => {
    mocks.cancelScope.mockImplementation(async () => {
      cancel.cancelScope();
    });
    mocks.search.mockImplementation(() => cancellable([OK_ITEM]));
    mocks.chapters.mockImplementation(() => cancellable(CHAPTERS_5));
    mocks.parse.mockImplementation(() => cancellable(PARSE_OK));

    const results = await Promise.all([
      switchSource("rule-b", CTX),
      switchSource("rule-b", CTX),
      switchSource("rule-b", CTX),
    ]);

    // cancelScope 仅用于作废旧 invocation：3 次并发调用中第 2/3 次各作废前一次
    // （Kimi K3 复审第 7 项；首次调用无前序，不触发）。
    expect(mocks.cancelScope).toHaveBeenCalledTimes(2);
    // 前两次为取消/竞态静默丢弃（status failed + discarded 标记 + 文案，不污染 lastError）
    expect(results[0].status).toBe("failed");
    expect(results[0].discarded).toBe(true);
    expect(results[0].message).toBe(SWITCH_SUPERSEDED_MESSAGE);
    expect(results[1].status).toBe("failed");
    expect(results[1].discarded).toBe(true);
    expect(results[1].message).toBe(SWITCH_SUPERSEDED_MESSAGE);
    expect(results[2].status).toBe("ok");
    expect(results[2].discarded).toBeUndefined();

    // store 只反映最后一次结果
    let state: { switching: boolean; currentScope: string | null; lastError: string | null } =
      { switching: true, currentScope: null, lastError: null };
    const unsub = sourceSwitchState.subscribe((s) => {
      state = s;
    });
    await new Promise((r) => setTimeout(r, 20));
    unsub();
    expect(state.switching).toBe(false);
    expect(state.lastError).toBeNull();
    // currentScope 是最后一次调用（第三次）的 invocation scope；callSeq 为模块级计数器，
    // 跨用例累积，无法断言具体数字，只断言其唯一性与格式。
    expect(state.currentScope).toMatch(/^play:c1:\d+$/);
  });

  it("switch_race_cancels_only_prev_invocation: 新调用只作废旧 invocation，不影响自身", async () => {
    // 按 invocation scope 隔离的取消模型（模拟 Rust 侧 scope 表）：cancelScope(inv) 只取消
    // 该 invocation 已注册的 token，其他 invocation 完全不受影响（Kimi K3 复审第 7 项）。
    const invTokens = new Map<string, Set<() => void>>();
    const track = (inv: string, reject: () => void) => {
      if (!invTokens.has(inv)) invTokens.set(inv, new Set());
      invTokens.get(inv)!.add(reject);
      return () => invTokens.get(inv)?.delete(reject);
    };
    const cancelInv = (inv: string) => {
      for (const r of invTokens.get(inv) ?? []) r();
      invTokens.delete(inv);
    };
    const cancellableInv = <T>(inv: string, value: T, delayMs: number): Promise<T> =>
      new Promise<T>((resolve, reject) => {
        const done = track(inv, () => reject(cancelledError()));
        setTimeout(() => {
          done();
          resolve(value);
        }, delayMs);
      });

    mocks.cancelScope.mockImplementation(async (scope: string) => {
      cancelInv(scope);
    });
    mocks.search.mockImplementation((_r: string, _k: string, _p: number, inv: string) =>
      cancellableInv(inv, [OK_ITEM], 5),
    );
    mocks.chapters.mockImplementation((_r: string, _u: string, inv: string) =>
      cancellableInv(inv, CHAPTERS_5, 5),
    );
    mocks.parse.mockImplementation((_r: string, _u: string, inv: string) =>
      cancellableInv(inv, PARSE_OK, 5),
    );

    const results = await Promise.all([
      switchSource("rule-b", CTX),
      switchSource("rule-b", CTX),
    ]);

    // A（旧）被 B 整体作废：search 收到 Cancelled → 静默丢弃
    expect(results[0].status).toBe("failed");
    expect(results[0].discarded).toBe(true);
    // B（最新）自身未被自己的 cancelScope 影响：正常 ok
    expect(results[1].status).toBe("ok");
    expect(results[1].parseResult?.urls[0]).toBe("https://cdn.example.com/v.m3u8");
  });

  it("switch_cancel_prev_failure_isolated: 作废旧调用取消失败静默降级，新切换照常成功（Kimi K3 复审第 8 项）", async () => {
    // 模拟 IPC 层取消旧 invocation 时抛错（如通道异常）。修复前会进入外层 catch 把
    // 本次全新切换判为 failed，且 lastError 展示旧调用的原始取消错误文案。
    mocks.cancelScope.mockRejectedValue(new Error("IPC 通道异常"));
    mocks.search.mockImplementation(() => new Promise((resolve) => setTimeout(() => resolve([OK_ITEM]), 1)));
    mocks.chapters.mockResolvedValue(CHAPTERS_5);
    mocks.parse.mockResolvedValue(PARSE_OK);

    const results = await Promise.all([
      switchSource("rule-b", CTX),
      switchSource("rule-b", CTX),
    ]);

    // 第二次（最新）调用尝试 cancelScope(前序 invocation)，失败被隔离 → 新切换照常 ok
    expect(mocks.cancelScope).toHaveBeenCalledTimes(1);
    expect(mocks.cancelScope.mock.calls[0][0]).toMatch(/^play:c1:\d+$/);
    expect(results[1].status).toBe("ok");
    expect(results[1].parseResult?.urls[0]).toBe("https://cdn.example.com/v.m3u8");
    expect(results[1].discarded).toBeUndefined();
    // 旧调用仍因 seq 校验被静默丢弃，不污染状态
    expect(results[0].status).toBe("failed");
    expect(results[0].discarded).toBe(true);

    // lastError 不得展示旧调用取消的原始错误，保持 null
    let state: { switching: boolean; currentScope: string | null; lastError: string | null } =
      { switching: true, currentScope: null, lastError: null };
    const unsub = sourceSwitchState.subscribe((s) => Object.assign(state, s));
    await new Promise((r) => setTimeout(r, 0));
    unsub();
    expect(state.lastError).toBeNull();
    expect(state.currentScope).toMatch(/^play:c1:\d+$/);
  });

  it("switch_race_late_parse_does_not_cancel_latest: 旧调用迟到的 parse 不得取消最新调用（Kimi K3 复审第 7 项）", async () => {
    // 复现竞态窗口：旧调用 A 的 search 已在取消信号到达前越过取消点（worker 已完成 JS），
    // 因此 A 继续走到 parse——A 的 parse 使用**独立** invocation scope，绝不能取消最新
    // 调用 B 已注册的 parse token。旧实现（parse 共享 "play:{contentId}" scope）下，
    // A 迟到的 parse 会通过 new_scope_token 把 B 的 token 取消，B 被误杀 → 两次切换全失败。
    const invTokens = new Map<string, Set<() => void>>();
    const track = (inv: string, reject: () => void) => {
      if (!invTokens.has(inv)) invTokens.set(inv, new Set());
      invTokens.get(inv)!.add(reject);
      return () => invTokens.get(inv)?.delete(reject);
    };
    const cancelInv = (inv: string) => {
      for (const r of invTokens.get(inv) ?? []) r();
      invTokens.delete(inv);
    };
    const cancellableInv = <T>(inv: string, value: T, delayMs: number): Promise<T> =>
      new Promise<T>((resolve, reject) => {
        const done = track(inv, () => reject(cancelledError()));
        setTimeout(() => {
          done();
          resolve(value);
        }, delayMs);
      });
    // 已越过取消点的操作：无视 cancelScope，照常完成（模拟 worker 内 JS 已结束/已注册新 token）
    const delayed = <T>(value: T, delayMs: number): Promise<T> =>
      new Promise<T>((resolve) => setTimeout(() => resolve(value), delayMs));

    mocks.cancelScope.mockImplementation(async (scope: string) => {
      cancelInv(scope);
    });
    let searchCalls = 0;
    mocks.search.mockImplementation((_r: string, _k: string, _p: number, inv: string) => {
      searchCalls += 1;
      // 第一次（旧调用 A）search 极慢且已越过取消点；最新调用 B 的 search 极快
      return searchCalls === 1 ? delayed([OK_ITEM], 40) : cancellableInv(inv, [OK_ITEM], 1);
    });
    mocks.chapters.mockImplementation((_r: string, _u: string, inv: string) =>
      cancellableInv(inv, CHAPTERS_5, 1),
    );
    mocks.parse.mockImplementation((_r: string, _u: string, inv: string) =>
      cancellableInv(inv, PARSE_OK, 1),
    );

    const results = await Promise.all([
      switchSource("rule-b", CTX),
      switchSource("rule-b", CTX),
    ]);

    // B（最新调用）的 parse 未被 A 迟到的 parse 取消：必须 ok
    expect(results[1].status).toBe("ok");
    expect(results[1].parseResult?.urls[0]).toBe("https://cdn.example.com/v.m3u8");
    expect(results[1].discarded).toBeUndefined();
    // A（旧调用）即便走完全程，也因 seq 校验被静默丢弃
    expect(results[0].status).toBe("failed");
    expect(results[0].discarded).toBe(true);
  });

  it("switch_failed_always_carries_message: 任何 failed 结果都有 message（spec §5 契约）", async () => {
    mocks.cancelScope.mockImplementation(async () => {
      cancel.cancelScope();
    });
    mocks.search.mockImplementation(() => cancellable([OK_ITEM]));
    mocks.chapters.mockImplementation(() => cancellable(CHAPTERS_5));
    mocks.parse.mockImplementation(() => cancellable(PARSE_OK));

    const results = await Promise.all([
      switchSource("rule-b", CTX),
      switchSource("rule-b", CTX),
    ]);

    // 两个失败（竞态丢弃）结果都必须携带 message，且与真实失败可区分（discarded 标记）
    for (const r of results.filter((x) => x.status === "failed")) {
      expect(r.message).toBeTruthy();
      expect(r.message!.length).toBeGreaterThan(0);
      expect(r.discarded).toBe(true);
    }
  });

  it("switch_result_to_playback: 丢弃结果透传 discarded，消费方可跳过 UI", () => {
    const playback = switchResultToPlayback({
      status: "failed",
      chapters: [],
      targetChapter: null,
      resumeSec: 0,
      message: SWITCH_SUPERSEDED_MESSAGE,
      discarded: true,
    });
    expect(playback.status).toBe("failed");
    expect(playback.message).toBe(SWITCH_SUPERSEDED_MESSAGE);
    expect(playback.discarded).toBe(true);
  });

  it("switch_never_throws: 解析失败转为结构化 failed，不抛异常", async () => {
    mocks.cancelScope.mockResolvedValue(undefined);
    mocks.search.mockResolvedValue([OK_ITEM]);
    mocks.chapters.mockResolvedValue(CHAPTERS_5);
    mocks.parse.mockRejectedValue(new Error("网络错误"));

    let result;
    try {
      result = await switchSource("rule-b", CTX);
    } catch {
      throw new Error("switchSource 不应向上抛异常");
    }

    expect(result.status).toBe("failed");
    let state: { lastError: string | null; lastResult: { status: string } | null } = {
      lastError: null,
      lastResult: null,
    };
    const unsub = sourceSwitchState.subscribe((s) => {
      state = s;
    });
    await new Promise((r) => setTimeout(r, 0));
    unsub();
    expect(state.lastError).toBeTruthy();
    expect(state.lastResult?.status).toBe("failed");
  });

  it("switch_rule_not_found_shows_friendly_message: RuleNotFound 转为可读文案（Kimi K3 非阻塞项）", async () => {
    mocks.cancelScope.mockResolvedValue(undefined);
    // Rust 侧 RuleExecError 的 kind tag 为 "ruleNotFound"（serde camelCase）；
    // describeSwitchError 必须归一化后匹配，不能因大小写漏判而落到原始 message。
    mocks.search.mockRejectedValue(
      Object.assign(new Error("规则不存在或无效: xxx"), { kind: "ruleNotFound" }),
    );

    const result = await switchSource("rule-b", CTX);

    expect(result.status).toBe("failed");
    let state: { lastError: string | null } = { lastError: null };
    const unsub = sourceSwitchState.subscribe((s) => {
      state = s;
    });
    await new Promise((r) => setTimeout(r, 0));
    unsub();
    expect(state.lastError).toBe("该源当前不可用或已被禁用，请选择其他源");
  });
});

describe("normalizeTitle", () => {
  it("全角/大小写/空白/标点变体归一化相等", () => {
    // 全角 → 半角 + 大小写
    expect(normalizeTitle("ＡＢＣ测试番")).toBe(normalizeTitle("abc测试番"));
    // 首尾空白
    expect(normalizeTitle("  进击的巨人 ")).toBe(normalizeTitle("进击的巨人"));
    // 全角标点
    expect(normalizeTitle("测试番！！")).toBe(normalizeTitle("测试番"));
  });

  it("保留字母数字与中文，去除标点", () => {
    expect(normalizeTitle("ONE PIECE — 海贼王！")).toBe("one piece 海贼王");
  });
});

// sourceSwitch store 单元测试（对应 spec §6.2 测试 15~19）
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
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

    expect(mocks.cancelScope).toHaveBeenCalledWith("play:c1");
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
    expect(playback.resumeSec).toBe(750);
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

    // 必须取归一化匹配的 detail_url，而非盲目用 items[0]
    expect(mocks.chapters).toHaveBeenCalledWith(
      "rule-b",
      "https://example.com/detail/right",
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

    // cancelScope 被调用 3 次
    expect(mocks.cancelScope).toHaveBeenCalledTimes(3);
    // 前两次为取消静默丢弃（status failed 且不污染 lastError）
    expect(results[0].status).toBe("failed");
    expect(results[1].status).toBe("failed");
    expect(results[2].status).toBe("ok");

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
    expect(state.currentScope).toBe("play:c1");
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

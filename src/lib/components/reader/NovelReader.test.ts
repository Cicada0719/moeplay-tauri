// NovelReader 组件测试（spec §6.2 C12）
import { fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import NovelReader from "./NovelReader.svelte";

vi.mock("../../history/historyApi", () => ({
  buildHistoryId: vi.fn(() => "mock-id"),
  upsertHistory: vi.fn(async () => {}),
}));

// eslint-disable-next-line import/first
import { upsertHistory } from "../../history/historyApi";

function mockScrollMetrics(
  container: HTMLElement,
  scrollHeight: number,
  clientHeight: number,
  scrollTop: number,
) {
  Object.defineProperty(container, "scrollHeight", { value: scrollHeight, configurable: true });
  Object.defineProperty(container, "clientHeight", { value: clientHeight, configurable: true });
  Object.defineProperty(container, "scrollTop", { value: scrollTop, configurable: true });
}

const BASE_PROPS = {
  contentId: "novel-1",
  sourceId: "s",
  chapterId: "ch30",
  chapterTitle: "第30章",
  content: "第一段落文字。\n\n第二段落的文字内容。",
};

beforeEach(() => {
  vi.mocked(upsertHistory).mockClear();
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe("NovelReader 滚动位置上报与恢复", () => {
  it("C12: scroll 至 45% → 防抖后 upsertHistory 收到 scrollPct≈0.45", async () => {
    render(NovelReader, { props: BASE_PROPS });
    const container = screen.getByTestId("novel-scroll");
    mockScrollMetrics(container, 1000, 500, 225); // 225 / (1000-500) = 0.45

    fireEvent.scroll(container);
    await waitFor(() => expect(upsertHistory).toHaveBeenCalled(), { timeout: 2000 });

    const item = vi.mocked(upsertHistory).mock.calls[0][0];
    expect(item.scrollPct).toBeCloseTo(0.45, 2);
    expect(item.contentType).toBe("novel");
    expect(item.chapterId).toBe("ch30");
  });

  it("C12b: initialScrollPct=0.45 进入 → 内容渲染完成后 scrollTo top≈0.45×(scrollHeight-clientHeight)（±5%）", async () => {
    render(NovelReader, { props: { ...BASE_PROPS, initialScrollPct: 0.45 } });
    const container = screen.getByTestId("novel-scroll");
    mockScrollMetrics(container, 1000, 500, 0);
    const scrollTo = vi.fn();
    container.scrollTo = scrollTo;

    await waitFor(() => expect(scrollTo).toHaveBeenCalled(), { timeout: 2000 });
    const arg = scrollTo.mock.calls[0][0];
    const expectedTop = 0.45 * (1000 - 500); // 225
    expect(Math.abs(arg.top - expectedTop) / expectedTop).toBeLessThan(0.05);
  });

  it("C12c: 分母 ≤0 时 scrollPct 记 0（除零保护），不抛异常", async () => {
    render(NovelReader, { props: BASE_PROPS });
    const container = screen.getByTestId("novel-scroll");
    mockScrollMetrics(container, 500, 500, 50); // scrollHeight - clientHeight = 0
    fireEvent.scroll(container);
    await waitFor(() => expect(upsertHistory).toHaveBeenCalled(), { timeout: 2000 });
    expect(vi.mocked(upsertHistory).mock.calls[0][0].scrollPct).toBe(0);
  });
});

describe("NovelReader keyed each 去重崩溃回归", () => {
  it("C12d: 含重复段落的文本渲染不崩溃，重复段落全部保留", () => {
    // 旧实现以段落文本作 each key，重复段落（如连续「——」）会触发
    // Svelte 5 duplicate-key 运行期错误；改为索引 key 后应全部渲染。
    const content = [
      "第一段。",
      "——",
      "——",
      "第二段。",
      "",
      "——",
      "第三段。",
    ].join("\n");
    render(NovelReader, { props: { ...BASE_PROPS, content } });

    const paragraphs = screen.getByTestId("novel-scroll").querySelectorAll("p");
    expect(paragraphs).toHaveLength(6);
    // 重复文本均渲染，不被去重丢弃。
    const texts = Array.from(paragraphs).map((p) => p.textContent);
    expect(texts.filter((t) => t === "——")).toHaveLength(3);
    expect(screen.queryByTestId("novel-empty")).not.toBeInTheDocument();
  });
});

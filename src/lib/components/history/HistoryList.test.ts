// HistoryList 组件测试（spec §6.2 C1~C6）
import { fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { ContentType, HistoryItem } from "../../history/types";
import HistoryList from "./HistoryList.svelte";

vi.mock("../../history/historyApi", () => ({
  queryHistory: vi.fn(),
  deleteHistory: vi.fn(async () => {}),
  clearHistoryByType: vi.fn(async () => {}),
  upsertHistory: vi.fn(async () => {}),
  buildHistoryId: vi.fn(() => "mock-id"),
  historyCache: { subscribe: vi.fn() },
}));

// eslint-disable-next-line import/first
import { clearHistoryByType, deleteHistory, queryHistory } from "../../history/historyApi";

function makeItem(id: number, contentType: ContentType, title: string): HistoryItem {
  return {
    id: String(id),
    contentId: `c${id}`,
    contentType,
    title,
    cover: null,
    sourceId: "src-a",
    chapterId: null,
    chapterTitle: `章节${id}`,
    pageIndex: id,
    positionSec: 0,
    scrollPct: 0,
    progress: id,
    updatedAt: Date.now() - id * 60_000,
    deviceId: "dev",
    deleted: false,
  };
}

const ALL_ITEMS: HistoryItem[] = [
  makeItem(1, "manga", "火影忍者"),
  makeItem(2, "manga", "海贼王"),
  makeItem(3, "anime", "进击的巨人"),
];

function installListMock(items: HistoryItem[] = ALL_ITEMS) {
  vi.mocked(queryHistory).mockImplementation(
    async ({ contentType } = {}) => {
      const filtered = contentType ? items.filter((i) => i.contentType === contentType) : items;
      return { items: filtered, total: filtered.length };
    },
  );
  vi.mocked(deleteHistory).mockClear();
  vi.mocked(clearHistoryByType).mockClear();
}

beforeEach(() => {
  installListMock();
});

describe("HistoryList 基础渲染与筛选", () => {
  it("C1: 渲染 3 条记录，Tab 切到「漫画」仅显示漫画项", async () => {
    render(HistoryList);
    await waitFor(() => expect(screen.getAllByTestId("history-row")).toHaveLength(3));

    await userEvent.click(screen.getByRole("tab", { name: "漫画" }));
    await waitFor(() => expect(screen.getAllByTestId("history-row")).toHaveLength(2));
    expect(screen.getAllByTestId("row-title").map((el) => el.textContent)).toEqual([
      "火影忍者",
      "海贼王",
    ]);
  });

  it("C2: 关键词 150ms 防抖后过滤，命中子串被 <mark> 包裹；无结果显空态", async () => {
    render(HistoryList);
    await waitFor(() => expect(screen.getAllByTestId("history-row")).toHaveLength(3));

    fireEvent.input(screen.getByTestId("search-input"), { target: { value: "火影" } });
    await waitFor(() => expect(screen.getAllByTestId("history-row")).toHaveLength(1));
    expect(screen.getByText("火影", { selector: "mark" })).toBeInTheDocument();

    fireEvent.input(screen.getByTestId("search-input"), { target: { value: "不存在的标题" } });
    await waitFor(() => expect(screen.getByTestId("history-search-empty")).toBeInTheDocument());
    expect(screen.queryByTestId("history-row")).not.toBeInTheDocument();
  });
});

describe("HistoryList 删除", () => {
  it("C3: 单条删除 → 确认后调用 deleteHistory([id]) 且行消失", async () => {
    render(HistoryList);
    await waitFor(() => expect(screen.getAllByTestId("history-row")).toHaveLength(3));

    await userEvent.click(screen.getAllByTestId("row-delete")[0]);
    await waitFor(() => expect(screen.getByRole("button", { name: "删除" })).toBeInTheDocument());
    await userEvent.click(screen.getByRole("button", { name: "删除" }));

    await waitFor(() => expect(deleteHistory).toHaveBeenCalledWith(["1"]));
    await waitFor(() => expect(screen.getAllByTestId("history-row")).toHaveLength(2));
  });

  it("C4: 批量模式勾选 2 条 → 删除 → 二次确认文案含 N=2 与同步提示", async () => {
    render(HistoryList);
    await waitFor(() => expect(screen.getAllByTestId("history-row")).toHaveLength(3));

    await userEvent.click(screen.getByRole("button", { name: "进入批量管理模式" }));
    const checks = screen.getAllByTestId("row-check");
    await userEvent.click(checks[0]);
    await userEvent.click(checks[1]);

    await userEvent.click(screen.getByRole("button", { name: /删除所选/ }));
    const message = screen.getByTestId("confirm-message");
    expect(message).toHaveTextContent(/将删除 2 条记录/);
    expect(message).toHaveTextContent(/同步后其他设备也会删除/);

    await userEvent.click(screen.getByRole("button", { name: /^删除所选$/ }));
    await waitFor(() => expect(deleteHistory).toHaveBeenCalledWith(["1", "2"]));
  });

  it("C5: 清空当前类型在 all Tab 下禁用；在漫画 Tab 下调用 clearHistoryByType('manga')", async () => {
    render(HistoryList);
    await waitFor(() => expect(screen.getAllByTestId("history-row")).toHaveLength(3));

    const clearBtn = screen.getByRole("button", { name: "清空当前类型" });
    expect(clearBtn).toBeDisabled();

    await userEvent.click(screen.getByRole("tab", { name: "漫画" }));
    await waitFor(() => expect(screen.getAllByTestId("history-row")).toHaveLength(2));
    expect(screen.getByRole("button", { name: "清空当前类型" })).toBeEnabled();

    await userEvent.click(screen.getByRole("button", { name: "清空当前类型" }));
    await waitFor(() =>
      expect(screen.getByTestId("confirm-message")).toHaveTextContent(/「漫画」历史记录/),
    );
    await userEvent.click(screen.getByRole("button", { name: "清空" }));
    await waitFor(() => expect(clearHistoryByType).toHaveBeenCalledWith("manga"));
  });
});

describe("HistoryList 虚拟滚动", () => {
  it("C6: 构造 10000 条数据，断言渲染的 DOM 行数 < 50", async () => {
    const many = Array.from({ length: 10000 }, (_, i) => makeItem(i, "manga", `漫画 第${i}话`));
    installListMock(many);
    render(HistoryList);

    await waitFor(() =>
      expect(screen.getAllByTestId("history-row").length).toBeGreaterThan(0),
    );
    const rendered = screen.getAllByTestId("history-row").length;
    expect(rendered).toBeGreaterThan(0);
    expect(rendered).toBeLessThan(50);
  });
});

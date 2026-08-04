import { describe, expect, it, vi } from "vitest";
import { render, screen, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import SourceSuggestSheet from "./SourceSuggestSheet.svelte";
import type { SourceInfo } from "../../services/sourceSwitch";

const sources: SourceInfo[] = [
  { id: "alpha", name: "Alpha", contentType: "anime", health: "degraded" },
  { id: "bravo", name: "Bravo", contentType: "anime", health: "unknown" },
  { id: "charlie", name: "Charlie", contentType: "anime", health: "ok" },
  { id: "delta", name: "Delta", contentType: "anime", health: "ok" },
];

function baseProps(overrides: Record<string, unknown> = {}) {
  return {
    sources,
    contentId: "anime-1",
    chapterId: "ep5",
    positionSec: 750,
    onSelect: vi.fn(),
    onClose: vi.fn(),
    ...overrides,
  };
}

describe("SourceSuggestSheet", () => {
  it("按健康度排序渲染（ok 在前），并展示健康标签", () => {
    render(SourceSuggestSheet, { props: baseProps() });

    const list = screen.getByTestId("source-suggest-sheet");
    const items = within(list)
      .getAllByRole("button")
      .filter((item) => item.getAttribute("data-source-id") !== null);
    expect(items.map((item) => item.getAttribute("data-source-id"))).toEqual(["charlie", "delta", "bravo", "alpha"]);
    expect(within(list).getAllByText("可用").length).toBeGreaterThan(0);
    expect(within(list).getByText("异常")).toBeInTheDocument();
  });

  it("携带 contentId / chapterId / positionSec 到列表数据属性", () => {
    render(SourceSuggestSheet, { props: baseProps() });
    const list = screen.getByTestId("source-suggest-sheet").querySelector("ul");
    expect(list).toHaveAttribute("data-content-id", "anime-1");
    expect(list).toHaveAttribute("data-chapter-id", "ep5");
    expect(list).toHaveAttribute("data-position-sec", "750");
  });

  it("点击条目调用 onSelect(sourceId)，关闭按钮调用 onClose", async () => {
    const onSelect = vi.fn();
    const onClose = vi.fn();
    render(SourceSuggestSheet, { props: baseProps({ onSelect, onClose }) });

    await userEvent.click(screen.getByRole("button", { name: /Charlie/ }));
    expect(onSelect).toHaveBeenCalledWith("charlie");

    await userEvent.click(screen.getByRole("button", { name: "关闭源推荐" }));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("空列表时显示占位文案", () => {
    render(SourceSuggestSheet, { props: baseProps({ sources: [] }) });
    expect(screen.getByText(/暂无可推荐源/)).toBeInTheDocument();
  });
});

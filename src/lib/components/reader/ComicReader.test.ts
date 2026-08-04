// ComicReader 组件测试（spec §6.2 C7~C11）
import { fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { mangaSettingsKey } from "../../stores/readerSettings";
import ComicReader from "./ComicReader.svelte";

vi.mock("../../history/historyApi", () => ({
  buildHistoryId: vi.fn(() => "mock-id"),
  upsertHistory: vi.fn(async () => {}),
}));

vi.mock("../../reader/imageMeta", () => ({
  probeImageSize: vi.fn(),
  getCachedImageSize: vi.fn(),
  clearImageSizeCache: vi.fn(),
}));

// eslint-disable-next-line import/first
import { upsertHistory } from "../../history/historyApi";
// eslint-disable-next-line import/first
import { probeImageSize } from "../../reader/imageMeta";

const sizeMap = new Map<string, { width: number; height: number }>();

function renderComic(
  props: {
    contentId: string;
    sourceId: string;
    chapterId: string;
    chapterTitle: string;
    pages: string[];
    initialPageIndex?: number;
    cover?: string | null;
  },
  settings?: { pageMode: "single" | "dual"; direction: "ltr" | "rtl"; forceNarrowDual: boolean },
) {
  if (settings) {
    localStorage.setItem(mangaSettingsKey(props.contentId), JSON.stringify(settings));
  } else {
    localStorage.removeItem(mangaSettingsKey(props.contentId));
  }
  return render(ComicReader, { props });
}

beforeEach(() => {
  sizeMap.clear();
  vi.mocked(probeImageSize).mockImplementation(async (url: string) => {
    return sizeMap.get(url) ?? { width: 100, height: 100 };
  });
  vi.mocked(upsertHistory).mockClear();
});

afterEach(() => {
  localStorage.clear();
  sizeMap.clear();
  vi.restoreAllMocks();
  Object.defineProperty(window, "innerWidth", { value: 1024, configurable: true });
});

describe("ComicReader 双页翻页方向", () => {
  it("C7: 双页 RTL 点击右侧热区前进一屏；→ 键同效；点击左侧/← 后退", async () => {
    const pages = Array.from({ length: 10 }, (_, i) => `p${i}`);
    renderComic(
      {
        contentId: "manga-1",
        sourceId: "s",
        chapterId: "ch1",
        chapterTitle: "第1话",
        pages,
        initialPageIndex: 0,
      },
      { pageMode: "dual", direction: "rtl", forceNarrowDual: true },
    );
    await waitFor(() => expect(screen.getByTestId("page-indicator")).toHaveTextContent("1-2 / 10"));

    await userEvent.click(screen.getByTestId("zone-right"));
    await waitFor(() => expect(screen.getByTestId("page-indicator")).toHaveTextContent("3-4 / 10"));

    fireEvent.keyDown(window, { key: "ArrowRight" });
    await waitFor(() => expect(screen.getByTestId("page-indicator")).toHaveTextContent("5-6 / 10"));

    fireEvent.keyDown(window, { key: "ArrowLeft" });
    await waitFor(() => expect(screen.getByTestId("page-indicator")).toHaveTextContent("3-4 / 10"));

    await userEvent.click(screen.getByTestId("zone-left"));
    await waitFor(() => expect(screen.getByTestId("page-indicator")).toHaveTextContent("1-2 / 10"));
  });
});

describe("ComicReader 跨页大图", () => {
  it("C8: 含跨页图时该屏仅渲染 1 个 <img>，页码指示总数不变", async () => {
    const pages = ["p0", "p1", "p2", "p3"];
    sizeMap.set("p1", { width: 2000, height: 1000 });
    renderComic(
      { contentId: "manga-2", sourceId: "s", chapterId: "ch1", chapterTitle: "第1话", pages },
      { pageMode: "dual", direction: "rtl", forceNarrowDual: true },
    );
    await waitFor(() => expect(screen.getByTestId("page-indicator")).toHaveTextContent("1 / 4"));

    await userEvent.click(screen.getByTestId("zone-right"));
    await waitFor(() => expect(screen.getByTestId("page-indicator")).toHaveTextContent("2 / 4"));

    const cells = screen.getAllByTestId("page-cell");
    expect(cells).toHaveLength(1);
    expect(cells[0].querySelector("img")).not.toBeNull();
  });
});

describe("ComicReader 窄窗口降级", () => {
  it("C9: window.innerWidth=700 切双页 → 出现「窗口过窄」提示；点「仍要双页」后双页生效", async () => {
    Object.defineProperty(window, "innerWidth", { value: 700, configurable: true });
    const pages = ["p0", "p1"];
    renderComic(
      { contentId: "manga-3", sourceId: "s", chapterId: "ch1", chapterTitle: "第1话", pages },
      { pageMode: "single", direction: "rtl", forceNarrowDual: false },
    );
    await waitFor(() => expect(screen.getByTestId("page-indicator")).toHaveTextContent("1 / 2"));

    await userEvent.click(screen.getByTestId("mode-toggle"));
    await waitFor(() => expect(screen.getByTestId("narrow-bar")).toBeInTheDocument());

    await userEvent.click(screen.getByTestId("narrow-force"));
    await waitFor(() =>
      expect(screen.getByTestId("comic-reader")).toHaveAttribute("data-page-mode", "dual"),
    );
    expect(screen.queryByTestId("narrow-bar")).not.toBeInTheDocument();

    // 双页生效且图片不裁剪（CSS `.page-cell img { object-fit: contain }` 保证；
    // happy-dom 不计算 Svelte 作用域样式，故以双页激活 + 两页渲染断言功能行为）
    const cells = screen.getAllByTestId("page-cell");
    expect(cells).toHaveLength(2);
    expect(cells[0].querySelector("img")).not.toBeNull();
    expect(screen.getByTestId("mode-toggle")).toHaveAttribute("aria-label", "切换到单页");
  });
});

describe("ComicReader 从历史恢复定位", () => {
  it("C10: initialPageIndex=6 进入 → 定位到含第 6 页的屏，不串页（R8 回归）", async () => {
    const pages = Array.from({ length: 10 }, (_, i) => `p${i}`);
    renderComic(
      {
        contentId: "manga-4",
        sourceId: "s",
        chapterId: "ch1",
        chapterTitle: "第1话",
        pages,
        initialPageIndex: 6,
      },
      { pageMode: "dual", direction: "rtl", forceNarrowDual: true },
    );
    await waitFor(() => expect(screen.getByTestId("page-indicator")).toHaveTextContent("7-8 / 10"));

    const cellImgs = Array.from(
      screen.getByTestId("reader-stage").querySelectorAll("img"),
    ).map((img) => img.getAttribute("data-page-index"));
    expect(cellImgs).toContain("6");
    expect(cellImgs).toContain("7");
  });
});

describe("ComicReader 位置上报", () => {
  it("C11: onDestroy 触发 flush，调用一次 upsertHistory 且 pageIndex 为当前 anchor", async () => {
    const pages = ["p0", "p1"];
    const { unmount } = renderComic(
      { contentId: "manga-5", sourceId: "s", chapterId: "ch1", chapterTitle: "第1话", pages },
      { pageMode: "dual", direction: "rtl", forceNarrowDual: true },
    );
    await waitFor(() => expect(screen.getByTestId("page-indicator")).toHaveTextContent("1-2 / 2"));

    unmount();
    expect(upsertHistory).toHaveBeenCalledTimes(1);
    const item = vi.mocked(upsertHistory).mock.calls[0][0];
    expect(item.pageIndex).toBe(0);
    expect(item.contentType).toBe("manga");
  });
});

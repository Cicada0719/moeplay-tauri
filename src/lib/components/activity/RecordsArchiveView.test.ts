import { fireEvent, render, screen } from "@testing-library/svelte";
import { describe, expect, it, vi } from "vitest";
import RecordsArchiveView from "./RecordsArchiveView.svelte";
import type { DashboardMediaActivity } from "./dashboard-model";

const items: DashboardMediaActivity[] = [
  { id: "game:1", kind: "game", title: "星海回声", subtitle: "游玩 2h 18m", timeLabel: "7/12 21:42", timestamp: Date.parse("2026-07-12T21:42:00Z"), imageSrc: null, payload: {} },
  { id: "anime:1", kind: "anime", title: "葬送的芙莉莲", subtitle: "看到 第 18 集", timeLabel: "7/12 01:14", timestamp: Date.parse("2026-07-12T01:14:00Z"), imageSrc: null, payload: {} },
];

describe("RecordsArchiveView", () => {
  it("renders the yearbook cover, mixed archive index and data inserts", () => {
    render(RecordsArchiveView, { props: {
      items,
      stats: [{ id: "playtime", label: "总游玩时长", value: "42h", detail: "8 个活跃日" }],
      dailyPoints: [{ key: "d1", label: "7/12", value: 3, valueLabel: "3 项" }],
      monthlyPoints: [{ key: "m1", label: "7月", value: 42, valueLabel: "42h" }],
      onOpen: vi.fn(), onImport: vi.fn(),
    } });
    expect(screen.getByRole("heading", { name: "ACTIVITY ARCHIVE" })).toBeInTheDocument();
    expect(screen.getByRole("list", { name: "个人媒体活动档案" })).toBeInTheDocument();
    expect(screen.getAllByText("星海回声").length).toBeGreaterThan(1);
    expect(screen.getByRole("heading", { name: "最近 14 天活动频率" })).toBeVisible();
    expect(screen.getByRole("heading", { name: "月度媒体档案" })).toBeVisible();
  });

  it("updates the media stage on focus and opens the selected activity", async () => {
    const onOpen = vi.fn();
    render(RecordsArchiveView, { props: { items, onOpen, onImport: vi.fn() } });
    const anime = screen.getByRole("button", { name: /葬送的芙莉莲/ });
    await fireEvent.focus(anime);
    expect(screen.getAllByText("葬送的芙莉莲").length).toBeGreaterThan(1);
    await fireEvent.click(anime);
    expect(onOpen).toHaveBeenCalledWith(items[1]);
  });
});



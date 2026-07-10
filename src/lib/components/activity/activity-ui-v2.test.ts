import { tick } from "svelte";
import { fireEvent, render, screen, within } from "@testing-library/svelte";
import { describe, expect, it, vi } from "vitest";
import type { ActivityEventView, ActivityStoreState } from "../../features/activity";
import ActivityEditorDialog from "./ActivityEditorDialog.svelte";
import ActivityV2Section from "./ActivityV2Section.svelte";
import BarChart from "./BarChart.svelte";
import StatBlock from "./StatBlock.svelte";

const activityEvent: ActivityEventView = {
  id: "event-1",
  resourceKind: "game",
  resourceId: "game-1",
  eventType: "progressed",
  startedAt: "2026-07-10T10:00:00.000Z",
  endedAt: "2026-07-10T11:00:00.000Z",
  durationSeconds: 3600,
  providerId: null,
  payload: {},
  durationQuality: "exact",
  sourceLegacyId: "legacy-1",
};

function activityState(): ActivityStoreState {
  return {
    filters: {},
    events: [activityEvent],
    summary: { eventCount: 2, exactDurationSeconds: 3600, estimatedDurationSeconds: 600, progressOnlyCount: 1, days: [] },
    continueCandidates: [{ resourceKind: "game", resourceId: "game-1", providerId: null, title: "星海回声", artworkUrl: null, position: {}, updatedAt: "2026-07-10T11:00:00.000Z", completed: false, durationQuality: "exact", exactDurationSeconds: 3600, estimatedDurationSeconds: null }],
    nextCursor: null,
    timelineGeneration: 1,
    continueGeneration: 1,
    isLoadingTimeline: false,
    isLoadingMore: false,
    isLoadingContinue: false,
    error: null,
  };
}

async function flushFocus() { await tick(); await Promise.resolve(); }

describe("activity UI-v2 patterns", () => {
  it("renders reusable StatBlock and a chart with an equivalent text summary", () => {
    render(StatBlock, { props: { label: "总游玩时长", value: "12.5h", detail: "4 个活跃日", tone: "accent" } });
    expect(screen.getByText("12.5h").closest("article")).toHaveAttribute("data-ui-pattern", "stat-block");

    render(BarChart, { props: { label: "最近两周", summary: "14 个时间点，合计 12.5h；峰值为 7/10 的 3.0h。", points: [{ key: "2026-07-10", label: "7/10", value: 10800, valueLabel: "3.0h" }] } });
    expect(screen.getByText("14 个时间点，合计 12.5h；峰值为 7/10 的 3.0h。")).toBeVisible();
    expect(screen.getByRole("img", { name: /最近两周。14 个时间点/ })).toBeInTheDocument();
  });

  it("renders Activity v2 with MediaRow continue/timeline patterns and forwards actions", async () => {
    const onEdit = vi.fn();
    const onContinue = vi.fn();
    const onFiltersChange = vi.fn();
    render(ActivityV2Section, { props: {
      state: activityState(), loaded: true, unavailable: false, exportPath: "events.json", exportFormat: "json",
      exactSeconds: 3600, estimatedSeconds: 600, progressOnlyEvents: 1,
      onFiltersChange, onClearFilters: vi.fn(), onContinue, onLoadMore: vi.fn(), onEdit, onDelete: vi.fn(), onExport: vi.fn(), onRetry: vi.fn(),
    } });

    expect(screen.getByRole("region", { name: "Activity v2 活动记录" })).toBeInTheDocument();
    await fireEvent.click(screen.getByRole("button", { name: "继续 星海回声" }));
    expect(onContinue).toHaveBeenCalledOnce();
    await fireEvent.click(screen.getByRole("button", { name: "编辑 game-1 活动" }));
    expect(onEdit).toHaveBeenCalledWith(activityEvent);
    await fireEvent.change(screen.getByLabelText("媒体"), { target: { value: "anime" } });
    expect(onFiltersChange).toHaveBeenCalledWith({ resourceKind: "anime" });
  });

  it("traps activity editor focus, handles Escape, restores the trigger, and emits a validated patch", async () => {
    const opener = document.createElement("button");
    opener.textContent = "编辑记录";
    document.body.append(opener);
    opener.focus();
    const onCancel = vi.fn();
    const onSave = vi.fn();
    const view = render(ActivityEditorDialog, { props: { event: activityEvent, onCancel, onSave } });
    await flushFocus();

    const dialog = screen.getByRole("dialog", { name: "编辑活动记录" });
    const startedAt = within(dialog).getByLabelText("开始时间");
    expect(document.activeElement).toBe(startedAt);
    const close = within(dialog).getByRole("button", { name: "关闭编辑器" });
    close.focus();
    await fireEvent.keyDown(close, { key: "Tab", shiftKey: true });
    expect(document.activeElement).toBe(within(dialog).getByRole("button", { name: "保存" }));

    await fireEvent.click(within(dialog).getByRole("button", { name: "保存" }));
    expect(onSave).toHaveBeenCalledWith(expect.objectContaining({ durationQuality: "exact", durationSeconds: 3600 }));

    await fireEvent.keyDown(document, { key: "Escape" });
    expect(onCancel).toHaveBeenCalledOnce();
    view.unmount();
    await flushFocus();
    expect(document.activeElement).toBe(opener);
    opener.remove();
  });
});




import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { fireEvent, render, screen, within } from "@testing-library/svelte";
import { describe, expect, it, vi } from "vitest";
import type { ContinueItem, ContinueStats } from "../stores/continue.svelte";
import ContinueCard from "./ContinueCard.svelte";
import ContinueHub from "./ContinueHub.svelte";

const game: ContinueItem = { id: "game-game-1", type: "game", title: "星海回声", cover: null, progress: 35, progressLabel: "2.0h", lastActivity: new Date("2026-07-10T10:00:00Z").getTime(), subtitle: "Fixture Studio", actionLabel: "继续游玩" };
const anime: ContinueItem = { id: "anime-anime-1", type: "anime", title: "夏日动画", cover: null, progress: 0, progressLabel: "第3话", lastActivity: new Date("2026-07-09T10:00:00Z").getTime(), subtitle: "本地源", actionLabel: "继续观看" };
const stats: ContinueStats = { totalCount: 2, gameCount: 1, animeCount: 1, comicCount: 0, todayMinutes: 120, weekMinutes: 360, streakDays: 4 };

describe("Continue UI-v2 migration", () => {
  it("renders ContinueCard as a keyboard-native MediaRow with progress semantics", async () => {
    const onclick = vi.fn();
    render(ContinueCard, { props: { item: game, onclick } });
    const button = screen.getByRole("button", { name: "继续 星海回声" });
    expect(button.closest("article")).toHaveAttribute("data-ui-v2", "media-row");
    expect(screen.getByRole("progressbar", { name: "星海回声 进度" })).toHaveAttribute("aria-valuenow", "35");
    await fireEvent.click(button);
    expect(onclick).toHaveBeenCalledOnce();
  });

  it("uses PageShell/PageHeader/AsyncSection/ContentGrid and filters supplied items without losing activation", async () => {
    const onSelect = vi.fn();
    render(ContinueHub, { props: { items: [game, anime], stats, topItem: game, onSelect } });
    expect(screen.getByRole("region", { name: "今日继续" })).toHaveAttribute("data-ui-v2", "page-shell");
    expect(screen.getByRole("heading", { level: 1, name: "今日继续" })).toBeInTheDocument();
    expect(screen.getAllByRole("region", { name: /今日概览|优先继续|继续列表/ })).toHaveLength(3);

    const filters = screen.getByRole("group", { name: "继续项目类型筛选" });
    await fireEvent.click(within(filters).getByRole("button", { name: /番剧/ }));
    expect(within(filters).getByRole("button", { name: /番剧/ })).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByRole("list", { name: "最近在看" })).toBeInTheDocument();

    await fireEvent.click(screen.getByRole("button", { name: "继续 夏日动画" }));
    expect(onSelect).toHaveBeenCalledWith(anime);
  });

  it("keeps the migrated production pages free of nested main and legacy Card shells", () => {
    const records = readFileSync(resolve(process.cwd(), "src/lib/components/PlayRecordsDashboard.svelte"), "utf8");
    const hub = readFileSync(resolve(process.cwd(), "src/lib/components/ContinueHub.svelte"), "utf8");
    for (const source of [records, hub]) {
      expect(source).toContain('PageShell');
      expect(source).toContain('as="div"');
      expect(source).toContain('PageHeader');
      expect(source).not.toContain('<main');
      expect(source).not.toContain('from "./ui/Card.svelte"');
    }
    expect(hub).toContain('AsyncSection');
    expect(readFileSync(resolve(process.cwd(), "src/lib/components/activity/ActivityV2Section.svelte"), "utf8")).toContain('AsyncSection');
    expect(readFileSync(resolve(process.cwd(), "src/lib/components/activity/LegacyOverviewSection.svelte"), "utf8")).toContain('AsyncSection');
  });
});


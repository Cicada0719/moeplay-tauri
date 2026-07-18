import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const root = process.cwd();
const replaySource = readFileSync(resolve(root, "src/lib/components/ReplayPage.svelte"), "utf8");
const drawerSource = readFileSync(resolve(root, "src/lib/components/CommandDrawer.svelte"), "utf8");
const i18nSource = readFileSync(resolve(root, "src/lib/stores/i18n.svelte.ts"), "utf8");

describe("replay page ui-v2 migration contract", () => {
	it("adopts the ui-v2 shell and header primitives with the grain layer", () => {
		expect(replaySource).toContain('from "./ui-v2"');
		expect(replaySource).toContain('<PageShell as="div"');
		expect(replaySource).toContain('width="full"');
		expect(replaySource).toContain("scrollable={false}");
		expect(replaySource).toContain('class="replay-v2-shell"');
		expect(replaySource).toContain('labelledBy="replay-page-title"');
		expect(replaySource).toContain("<PageHeader");
		expect(replaySource).toContain('class="v2-grain rp-grain"');
	});

	it("renders the bilingual eyebrow/title pairing and keeps the heading id", () => {
		expect(replaySource).toContain('eyebrow="回想 / REPLAY"');
		expect(replaySource).toContain('id="replay-page-title"');
		expect(replaySource).toContain('i18n.t("replay.title")');
		expect(replaySource).toContain('i18n.t("replay.subtitle")');
	});

	it("wires the year switcher with prev/next/back-to-this-year controls", () => {
		expect(replaySource).toContain("prevYear");
		expect(replaySource).toContain("nextYear");
		expect(replaySource).toContain("backToThisYear");
		expect(replaySource).toContain('i18n.t("replay.prev_year")');
		expect(replaySource).toContain('i18n.t("replay.next_year")');
		expect(replaySource).toContain('i18n.t("replay.this_year")');
	});

	it("derives every section from the replay aggregate feature module", () => {
		expect(replaySource).toContain('from "../features/replay/aggregate"');
		expect(replaySource).toContain("filterYearSessions(games, year)");
		expect(replaySource).toContain("summarizeYear(entries)");
		expect(replaySource).toContain("topPlayed(entries, 5)");
		expect(replaySource).toContain("monthlyHeat(entries)");
		expect(replaySource).toContain("topAchievements(games, 3)");
		expect(replaySource).toContain("topCompletions(games, 3)");
		expect(replaySource).toContain("newGamesTimeline(games, year)");
	});

	it("shows the empty state with a primary action navigating back to the library", () => {
		expect(replaySource).toContain("<EmptyState");
		expect(replaySource).toContain('i18n.t("replay.empty_title")');
		expect(replaySource).toContain('i18n.t("replay.empty_desc")');
		expect(replaySource).toContain('i18n.t("replay.empty_action")');
		expect(replaySource).toContain('uiStore.currentView = "home"');
	});

	it("gates all animation behind both reduced-motion signals", () => {
		expect(replaySource).toContain("prefers-reduced-motion: reduce");
		expect(replaySource).toContain('[data-motion="reduce"]');
	});

	it("avoids hardcoded colors in favor of theme tokens", () => {
		expect(replaySource).not.toMatch(/#[0-9a-fA-F]{3,8}\b/);
		expect(replaySource).not.toMatch(/rgba?\(/);
		expect(replaySource).toContain("var(--accent)");
		expect(replaySource).toContain("color-mix(in srgb, var(--accent)");
	});

	it("registers the replay view in the command drawer", () => {
		expect(drawerSource).toContain('{ id: "replay",       label: i18n.t("replay.nav"),     icon: "calendar" },');
		expect(drawerSource).toContain('if (v === "replay" && !ReplayPage) import("./ReplayPage.svelte")');
		expect(drawerSource).toContain('{:else if activeView === "replay" && ReplayPage}');
		expect(drawerSource).toContain("<ReplayPage />");
	});

	it("adds the replay i18n keys in both dictionaries", () => {
		expect(i18nSource).toContain('"replay.title": "年度游戏档案"');
		expect(i18nSource).toContain('"replay.title": "Year in Replay"');
		expect(i18nSource).toContain('"replay.nav": "年度回顾"');
		expect(i18nSource).toContain('"replay.nav": "Replay"');
		expect(i18nSource).toContain('"replay.oneliner": "这一年，你在 {games} 款游戏中度过了 {hours} 小时。"');
		expect(i18nSource).toContain('"replay.oneliner": "You spent {hours} hours across {games} games this year."');
		expect(i18nSource).toContain('"replay.empty_action": "回到游戏库"');
		expect(i18nSource).toContain('"replay.empty_action": "Back to library"');
	});
});

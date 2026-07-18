import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const root = process.cwd();
const statsSource = readFileSync(resolve(root, "src/lib/components/StatsPage.svelte"), "utf8");
const i18nSource = readFileSync(resolve(root, "src/lib/stores/i18n.svelte.ts"), "utf8");

describe("stats page ui-v2 migration contract", () => {
	it("adopts ui-v2 shell, header and state primitives", () => {
		expect(statsSource).toContain('from "./ui-v2"');
		expect(statsSource).toContain("PageShell");
		expect(statsSource).toContain("PageHeader");
		expect(statsSource).toContain("StateBoundary");
		expect(statsSource).toContain('<PageShell as="div"');
		expect(statsSource).toContain("<PageHeader");
		expect(statsSource).toContain("<StateBoundary");
		expect(statsSource).toContain('class="v2-grain st-grain"');
	});

	it("renders the bilingual eyebrow/title pairing and keeps the heading id", () => {
		expect(statsSource).toContain("統計 / STATS");
		expect(statsSource).toContain('id="stats-page-title"');
		expect(statsSource).toContain('i18n.t("stats.title")');
	});

	it("derives an explicit tri-state view model with retry wired to the loader", () => {
		expect(statsSource).toContain("viewState");
		expect(statsSource).toContain('"loading"');
		expect(statsSource).toContain('"empty"');
		expect(statsSource).toContain("onRetry={loadDashboard}");
	});

	it("keeps user behavior: collection cards filter and navigate home, status labels stay reactive", () => {
		expect(statsSource).toContain("handleCollectionClick");
		expect(statsSource).toContain("gameStore.filterTag");
		expect(statsSource).toContain('uiStore.currentView = "home"');
		expect(statsSource).toContain('i18n.t("stats.status_completed")');
	});

	it("gates the GSAP count animation behind both reduced-motion signals", () => {
		expect(statsSource).toContain('dataset.motion === "reduce"');
		expect(statsSource).toContain("prefers-reduced-motion: reduce");
		expect(statsSource).toContain('[data-motion="reduce"]');
	});

	it("drops the retired hand-rolled loading/error/empty scaffolding", () => {
		expect(statsSource).not.toContain("loading-stack");
		expect(statsSource).not.toContain("inline-error");
		expect(statsSource).not.toContain("empty-panel");
	});

	it("adds the new stats i18n keys in both dictionaries", () => {
		expect(i18nSource).toContain('"stats.title": "统计"');
		expect(i18nSource).toContain('"stats.title": "Stats"');
		expect(i18nSource).toContain('"stats.disk_hint"');
		expect(i18nSource).toContain('"stats.data_coverage"');
		expect(i18nSource).toContain('"stats.status_on_hold": "暂停"');
		expect(i18nSource).toContain('"stats.status_on_hold": "On hold"');
	});
});

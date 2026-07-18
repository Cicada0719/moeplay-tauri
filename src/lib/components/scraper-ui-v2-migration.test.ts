import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const root = process.cwd();
const scraperSource = readFileSync(resolve(root, "src/lib/components/ScraperPage.svelte"), "utf8");
const i18nSource = readFileSync(resolve(root, "src/lib/stores/i18n.svelte.ts"), "utf8");

describe("scraper page ui-v2 migration contract", () => {
	it("adopts ui-v2 shell, header and async state primitives", () => {
		expect(scraperSource).toContain('from "./ui-v2"');
		expect(scraperSource).toContain('<PageShell as="div"');
		expect(scraperSource).toContain("<PageHeader");
		expect(scraperSource).toContain("<AsyncState");
		expect(scraperSource).toContain('class="v2-grain sp-grain"');
	});

	it("renders the bilingual eyebrow/title pairing and keeps the heading id", () => {
		expect(scraperSource).toContain("スクレイプ / SCRAPER");
		expect(scraperSource).toContain('id="scraper-page-title"');
		expect(scraperSource).toContain('i18n.t("scraper.title")');
	});

	it("derives an explicit tri-state view model with retry wired to the scrape runner", () => {
		expect(scraperSource).toContain("scrapeState");
		expect(scraperSource).toContain('"loading"');
		expect(scraperSource).toContain('"error"');
		expect(scraperSource).toContain('"empty"');
		expect(scraperSource).toContain("onSelect: runScrape");
	});

	it("keeps user behavior: scrape run, source toggles, candidate selection and spotlight", () => {
		expect(scraperSource).toContain("scrapeGame");
		expect(scraperSource).toContain("runScrape");
		expect(scraperSource).toContain("toggleSource");
		expect(scraperSource).toContain("matchScore");
		expect(scraperSource).toContain("use:spotlight");
		expect(scraperSource).toContain('aria-label={i18n.t("scraper.results_aria")}');
	});

	it("drops the retired aura-era scaffolding", () => {
		expect(scraperSource).not.toContain("aura-page");
		expect(scraperSource).not.toContain("data-aura-echo");
		expect(scraperSource).not.toContain("aura-kicker");
		expect(scraperSource).not.toContain("aura-num");
		expect(scraperSource).not.toContain("aura-title");
		expect(scraperSource).not.toContain("aura-panel--spot");
		expect(scraperSource).not.toContain("--aura-line");
	});

	it("gates motion behind both reduced-motion signals", () => {
		expect(scraperSource).toContain("prefers-reduced-motion: reduce");
		expect(scraperSource).toContain('[data-motion="reduce"]');
	});

	it("keeps the scraper i18n keys in both dictionaries", () => {
		expect(i18nSource).toContain('"scraper.title": "AI 刮削中心"');
		expect(i18nSource).toContain('"scraper.title": "AI Scraper"');
		expect(i18nSource).toContain('"scraper.strategy_patch"');
		expect(i18nSource).toContain('"scraper.idle_title"');
		expect(i18nSource).toContain('"scraper.error_title"');
	});
});

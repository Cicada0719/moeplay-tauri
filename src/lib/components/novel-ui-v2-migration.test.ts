import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const root = process.cwd();
const novelSource = readFileSync(resolve(root, "src/lib/components/NovelPage.svelte"), "utf8");
const i18nSource = readFileSync(resolve(root, "src/lib/stores/i18n.svelte.ts"), "utf8");

describe("novel page ui-v2 migration contract", () => {
	it("adopts ui-v2 shell, header, filter bar and async state primitives", () => {
		expect(novelSource).toContain('from "./ui-v2"');
		expect(novelSource).toContain('<PageShell as="div"');
		expect(novelSource).toContain("<PageHeader");
		expect(novelSource).toContain("<FilterBar");
		expect(novelSource).toContain("<AsyncState");
		expect(novelSource).toContain('class="v2-grain nv-grain"');
	});

	it("renders the bilingual eyebrow/title pairing on the home view", () => {
		expect(novelSource).toContain("小説 / NOVEL");
		expect(novelSource).toContain('id="novel-page-title"');
		expect(novelSource).toContain('i18n.t("novel.title")');
	});

	it("keeps router hooks consumed outside the page", () => {
		expect(novelSource).toContain("data-route-scroll");
		expect(novelSource).toContain('data-search-scope="novel"');
		expect(novelSource).toContain('data-testid="novel-page"');
	});

	it("keeps user behavior: search, source switch, reader progress and epub download", () => {
		expect(novelSource).toContain("submitSearch");
		expect(novelSource).toContain("selectSource");
		expect(novelSource).toContain("saveReaderProgress");
		expect(novelSource).toContain("downloadBook");
		expect(novelSource).toContain('{ id: "x80"');
		expect(novelSource).toContain('{ id: "internetarchive"');
		expect(novelSource).toContain('{ id: "openlibrary"');
		expect(novelSource).toContain('{ id: "standardebooks"');
		expect(novelSource).toContain("drainSearchQueue");
		expect(novelSource).toContain("selectedSource");
		expect(novelSource).not.toContain('data-gamepad-activate="下载 EPUB"');
		expect(novelSource).toContain("moveChapter");
	});

	it("drops the retired hand-rolled loading/empty scaffolding", () => {
		expect(novelSource).not.toContain("loading-state");
	});

	it("gates spinner and smooth scrolling behind both reduced-motion signals", () => {
		expect(novelSource).toContain("prefers-reduced-motion: reduce");
		expect(novelSource).toContain('[data-motion="reduce"]) .spinner');
		expect(novelSource).toContain('[data-motion="reduce"]) .reader-scroll');
	});

	it("adds the new novel i18n keys in both dictionaries", () => {
		expect(i18nSource).toContain('"novel.title": "小说阅读"');
		expect(i18nSource).toContain('"novel.title": "Novels"');
		expect(i18nSource).toContain('"novel.source_all"');
		expect(i18nSource).toContain('"novel.source_x80": "80小说网"');
		expect(i18nSource).toContain('"novel.rights_x80"');
		expect(i18nSource).toContain('"novel.intro_x80_title"');
		expect(i18nSource).toContain('"novel.no_results_title"');
		expect(i18nSource).toContain('"novel.epub_failed"');
		expect(i18nSource).toContain('"novel.rights_gutenberg"');
	});
});

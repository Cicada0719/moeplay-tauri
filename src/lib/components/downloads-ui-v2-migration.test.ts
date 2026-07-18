import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const root = process.cwd();
const downloadsSource = readFileSync(resolve(root, "src/lib/components/DownloadPage.svelte"), "utf8");
const i18nSource = readFileSync(resolve(root, "src/lib/stores/i18n.svelte.ts"), "utf8");

describe("downloads page ui-v2 migration contract", () => {
	it("adopts ui-v2 shell, header, filter bar and async state primitives", () => {
		expect(downloadsSource).toContain('from "./ui-v2"');
		expect(downloadsSource).toContain("PageShell");
		expect(downloadsSource).toContain("PageHeader");
		expect(downloadsSource).toContain("FilterBar");
		expect(downloadsSource).toContain("AsyncState");
		expect(downloadsSource).toContain("<PageShell");
		expect(downloadsSource).toContain("<PageHeader");
		expect(downloadsSource).toContain("<FilterBar");
		expect(downloadsSource).toContain("<AsyncState");
		expect(downloadsSource).toContain('class="v2-grain dl-grain"');
	});

	it("renders the bilingual eyebrow/title pairing and keeps the heading id", () => {
		expect(downloadsSource).toContain("ダウンロード / DOWNLOADS");
		expect(downloadsSource).toContain('id="downloads-page-title"');
		expect(downloadsSource).toContain('i18n.t("downloads.title")');
	});

	it("derives explicit tri-state view models with an initial-load gate", () => {
		expect(downloadsSource).toContain("generalViewState");
		expect(downloadsSource).toContain("animeViewState");
		expect(downloadsSource).toContain("initialLoading");
	});

	it("keeps user behavior: add, row actions, clear-finished and empty-state focus", () => {
		expect(downloadsSource).toContain("press={start}");
		expect(downloadsSource).toContain("act(");
		expect(downloadsSource).toContain("removeRow(");
		expect(downloadsSource).toContain("clearGeneralFinished");
		expect(downloadsSource).toContain("focusUrlInput");
		expect(downloadsSource).toContain("onSelect: focusUrlInput");
	});

	it("drops the retired dead scaffolding", () => {
		expect(downloadsSource).not.toContain("data-job-state");
		expect(downloadsSource).not.toContain("EmptyState");
		expect(downloadsSource).not.toContain("inline-error");
	});

	it("keeps progress-bar animation gated behind both reduced-motion signals", () => {
		expect(downloadsSource).toContain("prefers-reduced-motion: reduce");
		expect(downloadsSource).toContain('[data-motion="reduce"]');
	});

	it("adds the new downloads i18n keys in both dictionaries", () => {
		expect(i18nSource).toContain('"downloads.title": "资源下载"');
		expect(i18nSource).toContain('"downloads.title": "Downloads"');
		expect(i18nSource).toContain('"download.status.parsing": "解析中"');
		expect(i18nSource).toContain('"download.status.parsing": "Parsing"');
		expect(i18nSource).toContain('"downloads.badge_recovered"');
		expect(i18nSource).toContain('"downloads.preflight_fail"');
		expect(i18nSource).toContain('"downloads.empty_action"');
		expect(i18nSource).toContain('"downloads.anime_empty_title"');
	});
});

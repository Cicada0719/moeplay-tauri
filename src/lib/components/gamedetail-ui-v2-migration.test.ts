import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const root = process.cwd();
const detailSource = readFileSync(resolve(root, "src/lib/components/GameDetailPage.svelte"), "utf8");
const i18nSource = readFileSync(resolve(root, "src/lib/stores/i18n.svelte.ts"), "utf8");

describe("game detail page ui-v2 migration contract", () => {
	it("stays on the ui-v2 detail panel and async state primitives", () => {
		expect(detailSource).toContain('from "./ui-v2"');
		expect(detailSource).toContain("<DetailPanel");
		expect(detailSource).toContain("<AsyncState");
	});

	it("keeps the cinematic archive layout and router/test hooks untouched", () => {
		expect(detailSource).toContain('data-testid="game-detail-page"');
		expect(detailSource).toContain('class="hero-contact-sheet"');
		expect(detailSource).toContain("width: 100vw");
		expect(detailSource).toContain('initialFocus=".game-detail-primary"');
		expect(detailSource).toContain("<SavePanel");
		expect(detailSource).toContain("<GameNotes");
	});

	it("keeps user behavior: launch, jp launch and the scrape dialog", () => {
		expect(detailSource).toContain("handleLaunch");
		expect(detailSource).toContain("handleLaunchJP");
		expect(detailSource).toContain("openScrapeDialog");
	});

	it("persists launch edits without replacing the whole game and protects long titles", () => {
		expect(detailSource).toContain("updateExePath(game.id, nextExePath)");
		expect(detailSource).toContain("updateInstallDir(game.id, nextInstallDir)");
		expect(detailSource).toContain("updateGameName(game.id, nextName)");
		expect(detailSource).not.toContain("await updateGame({ ...game");
		expect(detailSource).toContain("-webkit-line-clamp: 2");
		expect(detailSource).toContain("overflow-wrap: anywhere");
	});

	it("wires page copy through the gamedetail i18n prefix", () => {
		expect(detailSource).toContain('i18n.t("gamedetail.missing_title")');
		expect(detailSource).toContain('i18n.t("gamedetail.launch")');
		expect(detailSource).toContain('i18n.t("gamedetail.synopsis_empty")');
		expect(detailSource).toContain('i18n.t("gamedetail.edit_title"');
		expect(detailSource).toContain('i18n.t("gamedetail.achievements_none")');
	});

	it("gates the GSAP entrance behind both reduced-motion signals", () => {
		expect(detailSource).toContain('prefers-reduced-motion: reduce)');
		expect(detailSource).toContain('document.documentElement.dataset.motion === "reduce"');
	});

	it("writes reduced-motion fallbacks for both signals in styles", () => {
		expect(detailSource).toContain("@media (prefers-reduced-motion: reduce)");
		expect(detailSource).toContain('[data-motion="reduce"]) .bg-layer');
	});

	it("adds the new gamedetail i18n keys in both dictionaries", () => {
		expect(i18nSource).toContain('"gamedetail.missing_title": "游戏未找到"');
		expect(i18nSource).toContain('"gamedetail.missing_title": "Game Not Found"');
		expect(i18nSource).toContain('"gamedetail.sessions_empty"');
		expect(i18nSource).toContain('"gamedetail.mobile_note"');
	});
});

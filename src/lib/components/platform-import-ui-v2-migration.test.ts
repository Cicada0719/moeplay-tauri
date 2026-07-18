import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const root = process.cwd();
const importSource = readFileSync(resolve(root, "src/lib/components/PlatformImportPage.svelte"), "utf8");
const i18nSource = readFileSync(resolve(root, "src/lib/stores/i18n.svelte.ts"), "utf8");

describe("platform import page ui-v2 migration contract", () => {
	it("adopts ui-v2 shell, header and state boundary primitives", () => {
		expect(importSource).toContain('from "./ui-v2"');
		expect(importSource).toContain('<PageShell as="div"');
		expect(importSource).toContain("<PageHeader");
		expect(importSource).toContain("<StateBoundary");
		expect(importSource).toContain('class="v2-grain pi-grain"');
	});

	it("renders the bilingual eyebrow/title pairing and keeps the heading id", () => {
		expect(importSource).toContain("インポート / IMPORT");
		expect(importSource).toContain('id="platform-import-page-title"');
		expect(importSource).toContain('i18n.t("platform_import.title")');
	});

	it("derives an explicit tri-state view model with retry wired to the initial loader", () => {
		expect(importSource).toContain("statusViewState");
		expect(importSource).toContain('"loading"');
		expect(importSource).toContain('"error"');
		expect(importSource).toContain("onRetry={loadInitialState}");
	});

	it("keeps the Steam achievement sync panel inside the account section", () => {
		expect(importSource).toContain("achievement-sync-row");
		expect(importSource).toContain("handleSyncAchievements");
		expect(importSource).toContain('i18n.t("platform_import.achievement_sync")');
		expect(importSource).toContain('i18n.t("platform_import.achievement_result"');
	});

	it("keeps user behavior: scan, login, import flows and aggregate sync", () => {
		expect(importSource).toContain("scanPlatformLibrary");
		expect(importSource).toContain("importPlatformLibrary");
		expect(importSource).toContain("importSteamSessionGames");
		expect(importSource).toContain("syncSteamAchievements");
		expect(importSource).toContain("importAllAvailable");
		expect(importSource).toContain("LibraryV2ImportPanel");
	});

	it("drops the retired aura-era scaffolding and dead backdrop", () => {
		expect(importSource).not.toContain("aura-page");
		expect(importSource).not.toContain("data-aura-echo");
		expect(importSource).not.toContain("aura-kicker");
		expect(importSource).not.toContain("defaultLibraryBackdrop");
		expect(importSource).not.toContain("loading-cover");
	});

	it("gates the progress animation behind both reduced-motion signals", () => {
		expect(importSource).toContain("prefers-reduced-motion: reduce");
		expect(importSource).toContain('[data-motion="reduce"]');
	});

	it("adds the new platform_import i18n keys in both dictionaries", () => {
		expect(i18nSource).toContain('"platform_import.title": "平台导入"');
		expect(i18nSource).toContain('"platform_import.title": "Platform Import"');
		expect(i18nSource).toContain('"platform_import.achievement_result"');
		expect(i18nSource).toContain('"platform_import.empty_waiting_title"');
		expect(i18nSource).toContain('"platform_import.candidates_epic"');
	});
});

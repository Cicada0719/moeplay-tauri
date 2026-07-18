import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const root = process.cwd();
const wizardSource = readFileSync(resolve(root, "src/lib/components/FirstRunWizard.svelte"), "utf8");
const i18nSource = readFileSync(resolve(root, "src/lib/stores/i18n.svelte.ts"), "utf8");

describe("first-run wizard ui-v2 migration contract", () => {
	it("retires the aura skin entirely", () => {
		expect(wizardSource).not.toContain("aura-page");
		expect(wizardSource).not.toContain("aura-panel");
		expect(wizardSource).not.toContain("aura-bevel");
		expect(wizardSource).not.toContain("aura-head");
		expect(wizardSource).not.toContain("aura-kicker");
		expect(wizardSource).not.toContain("aura-title");
		expect(wizardSource).not.toContain("aura-num");
		expect(wizardSource).not.toContain("data-aura-echo");
		expect(wizardSource).not.toContain("--aura-");
	});

	it("keeps the dialog semantics and wizard overlay structure", () => {
		expect(wizardSource).toContain('role="dialog"');
		expect(wizardSource).toContain("wizard-overlay");
		expect(wizardSource).toContain('tabindex="-1"');
		expect(wizardSource).toContain('role="progressbar"');
		expect(wizardSource).toContain("wizard-head");
		expect(wizardSource).toContain("step-num");
	});

	it("derives step titles/descriptions from the firstrun i18n keys", () => {
		expect(wizardSource).toContain('i18n.t("firstrun.welcome_title")');
		expect(wizardSource).toContain('i18n.t("firstrun.welcome_desc")');
		expect(wizardSource).toContain('i18n.t("firstrun.ai_title")');
		expect(wizardSource).toContain('i18n.t("firstrun.ai_desc")');
		expect(wizardSource).toContain('i18n.t("firstrun.sources_title")');
		expect(wizardSource).toContain('i18n.t("firstrun.sources_desc")');
	});

	it("wires the remaining template copy through i18n", () => {
		expect(wizardSource).toContain('i18n.t("firstrun.overlay_aria")');
		expect(wizardSource).toContain('i18n.t("firstrun.step_aria")');
		expect(wizardSource).toContain('i18n.t("firstrun.dir_hint")');
		expect(wizardSource).toContain('i18n.t("firstrun.pick_folder")');
		expect(wizardSource).toContain('i18n.t("firstrun.remove_aria")');
		expect(wizardSource).toContain('i18n.t("firstrun.scanning_dirs", { count: scanDirs.length })');
		expect(wizardSource).toContain('i18n.t("firstrun.import_result", { imported: scanResult.imported, skipped: scanResult.skipped })');
		expect(wizardSource).toContain('i18n.t("firstrun.candidates_found", { count: candidates.length })');
		expect(wizardSource).toContain('i18n.t("firstrun.select_all")');
		expect(wizardSource).toContain('i18n.t("firstrun.dup")');
		expect(wizardSource).toContain('i18n.t("firstrun.none_found")');
		expect(wizardSource).toContain('i18n.t("firstrun.skip")');
		expect(wizardSource).toContain('i18n.t("firstrun.rescan")');
		expect(wizardSource).toContain('i18n.t("firstrun.import_selected")');
		expect(wizardSource).toContain('i18n.t("firstrun.scan_preview")');
		expect(wizardSource).toContain('i18n.t("firstrun.next")');
		expect(wizardSource).toContain('i18n.t("firstrun.prev")');
		expect(wizardSource).toContain('i18n.t("firstrun.api_url")');
		expect(wizardSource).toContain('i18n.t("firstrun.api_key_configured")');
		expect(wizardSource).toContain('i18n.t("firstrun.api_key_missing")');
		expect(wizardSource).toContain('i18n.t("firstrun.model")');
		expect(wizardSource).toContain('i18n.t("firstrun.finish_local")');
		expect(wizardSource).toContain('i18n.t("firstrun.add_local")');
		expect(wizardSource).toContain('i18n.t("firstrun.finish_local_desc")');
		expect(wizardSource).toContain('i18n.t("firstrun.finish_platform")');
		expect(wizardSource).toContain('i18n.t("firstrun.finish_platform_desc")');
		expect(wizardSource).toContain('i18n.t("firstrun.finish_aria")');
		expect(wizardSource).toContain('i18n.t("firstrun.saving")');
		expect(wizardSource).toContain('i18n.t("firstrun.start")');
	});

	it("keeps user behavior: directory pick, preview, import, secret and finish flows", () => {
		expect(wizardSource).toContain("pickDirectory");
		expect(wizardSource).toContain("previewDirectoryForGames");
		expect(wizardSource).toContain("importSelectedCandidates");
		expect(wizardSource).toContain("secretSet");
		expect(wizardSource).toContain("saveAndFinish");
	});

	it("keeps the fadeInScale entry animation and the clipped corner card", () => {
		expect(wizardSource).toContain("fadeInScale");
		expect(wizardSource).toContain("clip-path: polygon");
	});

	it("gates every animation and transition behind both reduced-motion signals", () => {
		expect(wizardSource).toContain("prefers-reduced-motion: reduce");
		expect(wizardSource).toContain('[data-motion="reduce"]');
	});

	it("ships the firstrun keys in both dictionaries", () => {
		expect(i18nSource).toContain('"firstrun.welcome_title": "欢迎使用萌游"');
		expect(i18nSource).toContain('"firstrun.welcome_title": "Welcome to MoeGame"');
		expect(i18nSource).toContain('"firstrun.start"');
		expect(i18nSource).toContain('"firstrun.finish_platform"');
	});
});

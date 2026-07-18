import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const root = process.cwd();
const emulatorSource = readFileSync(resolve(root, "src/lib/components/EmulatorImportDialog.svelte"), "utf8");
const i18nSource = readFileSync(resolve(root, "src/lib/stores/i18n.svelte.ts"), "utf8");

describe("emulator import dialog ui-v2 migration contract", () => {
	it("adopts ui-v2 shell, header and async state primitives", () => {
		expect(emulatorSource).toContain('from "./ui-v2"');
		expect(emulatorSource).toContain('<PageShell as="div"');
		expect(emulatorSource).toContain("<PageHeader");
		expect(emulatorSource).toContain("<AsyncState");
		expect(emulatorSource).toContain('class="v2-grain ei-grain"');
	});

	it("renders the bilingual eyebrow/title pairing and keeps the heading id", () => {
		expect(emulatorSource).toContain("エミュレータ / EMULATOR");
		expect(emulatorSource).toContain('id="emulator-import-page-title"');
		expect(emulatorSource).toContain('i18n.t("emulator_import.title")');
	});

	it("keeps the escape-to-close behavior and the home-bound close", () => {
		expect(emulatorSource).toContain('e.key === "Escape"');
		expect(emulatorSource).toContain("function close()");
		expect(emulatorSource).toContain('uiStore.currentView = "home"');
		expect(emulatorSource).toContain('i18n.t("emulator_import.close_aria")');
	});

	it("derives an explicit tri-state view model for the emulator scan", () => {
		expect(emulatorSource).toContain("emuScanState");
		expect(emulatorSource).toContain('"loading"');
		expect(emulatorSource).toContain('"empty"');
		expect(emulatorSource).toContain('"ready"');
	});

	it("keeps user behavior: emulator scan, rom scan, import and selection helpers", () => {
		expect(emulatorSource).toContain("doScanEmulators");
		expect(emulatorSource).toContain("pickEmuFolder");
		expect(emulatorSource).toContain("doScanRoms");
		expect(emulatorSource).toContain("doImport");
		expect(emulatorSource).toContain("toggleAllRoms");
		expect(emulatorSource).toContain("toggleOneRom");
		expect(emulatorSource).toContain("formatSize");
	});

	it("drops the retired overlay dialog and aura-era scaffolding", () => {
		expect(emulatorSource).not.toContain("aura-page");
		expect(emulatorSource).not.toContain("data-aura-echo");
		expect(emulatorSource).not.toContain("aura-kicker");
		expect(emulatorSource).not.toContain("aura-num");
		expect(emulatorSource).not.toContain("aura-title");
		expect(emulatorSource).not.toContain("aura-empty");
		expect(emulatorSource).not.toContain("aura-section");
		expect(emulatorSource).not.toContain("aura-bevel");
		expect(emulatorSource).not.toContain('role="dialog"');
		expect(emulatorSource).not.toContain("position: fixed");
	});

	it("gates motion behind both reduced-motion signals", () => {
		expect(emulatorSource).toContain("prefers-reduced-motion: reduce");
		expect(emulatorSource).toContain('[data-motion="reduce"]');
	});

	it("keeps the emulator_import i18n keys in both dictionaries", () => {
		expect(i18nSource).toContain('"emulator_import.title": "模拟器与 ROM 导入"');
		expect(i18nSource).toContain('"emulator_import.title": "Emulator & ROM Import"');
		expect(i18nSource).toContain('"emulator_import.scan_prompt"');
		expect(i18nSource).toContain('"emulator_import.import_count"');
	});
});

import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const root = process.cwd();
const diagnosticsSource = readFileSync(resolve(root, "src/lib/components/DiagnosticsPage.svelte"), "utf8");
const i18nSource = readFileSync(resolve(root, "src/lib/stores/i18n.svelte.ts"), "utf8");

describe("diagnostics page ui-v2 migration contract", () => {
	it("adopts ui-v2 shell, header and state primitives", () => {
		expect(diagnosticsSource).toContain('from "./ui-v2"');
		expect(diagnosticsSource).toContain('<PageShell as="div"');
		expect(diagnosticsSource).toContain("<PageHeader");
		expect(diagnosticsSource).toContain("<StateBoundary");
		expect(diagnosticsSource).toContain('class="v2-grain dg-grain"');
	});

	it("renders the bilingual eyebrow/title pairing and keeps the heading id", () => {
		expect(diagnosticsSource).toContain("診断 / DIAGNOSTICS");
		expect(diagnosticsSource).toContain('id="diagnostics-page-title"');
		expect(diagnosticsSource).toContain('i18n.t("diagnostics.title")');
	});

	it("derives an explicit tri-state view model with retry wired to the loader", () => {
		expect(diagnosticsSource).toContain("viewState");
		expect(diagnosticsSource).toContain('"loading"');
		expect(diagnosticsSource).toContain("onRetry={load}");
	});

	it("keeps user behavior: diagnostics run and redacted export", () => {
		expect(diagnosticsSource).toContain("runDiagnostics");
		expect(diagnosticsSource).toContain("exportBundle");
		expect(diagnosticsSource).toContain("exportDiagnosticsZip");
	});

	it("surfaces export failures through a dedicated banner instead of the swallowed shared error", () => {
		expect(diagnosticsSource).toContain("exportError");
		expect(diagnosticsSource).toContain('i18n.t("diagnostics.export_failed"');
		expect(diagnosticsSource).toContain('role="alert"');
	});

	it("drops the retired aura-era scaffolding", () => {
		expect(diagnosticsSource).not.toContain("aura-page");
		expect(diagnosticsSource).not.toContain("data-aura-echo");
		expect(diagnosticsSource).not.toContain("aura-inset");
	});

	it("gates smooth scrolling behind both reduced-motion signals", () => {
		expect(diagnosticsSource).toContain("prefers-reduced-motion: reduce");
		expect(diagnosticsSource).toContain('[data-motion="reduce"]');
	});

	it("adds the new diagnostics i18n keys in both dictionaries", () => {
		expect(i18nSource).toContain('"diagnostics.title": "诊断"');
		expect(i18nSource).toContain('"diagnostics.title": "Diagnostics"');
		expect(i18nSource).toContain('"diagnostics.export_failed"');
		expect(i18nSource).toContain('"diagnostics.migration_pending"');
	});
});

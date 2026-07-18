import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const root = process.cwd();
const backupSource = readFileSync(resolve(root, "src/lib/components/BackupPage.svelte"), "utf8");
const i18nSource = readFileSync(resolve(root, "src/lib/stores/i18n.svelte.ts"), "utf8");

describe("backup page ui-v2 migration contract", () => {
	it("adopts ui-v2 shell, header, filter bar and state primitives", () => {
		expect(backupSource).toContain('from "./ui-v2"');
		expect(backupSource).toContain('<PageShell as="div"');
		expect(backupSource).toContain("<PageHeader");
		expect(backupSource).toContain("<FilterBar");
		expect(backupSource).toContain("<StateBoundary");
		expect(backupSource).toContain('class="v2-grain bk-grain"');
	});

	it("renders the bilingual eyebrow/title pairing and keeps the heading id", () => {
		expect(backupSource).toContain("バックアップ / BACKUP");
		expect(backupSource).toContain('id="backup-page-title"');
		expect(backupSource).toContain('i18n.t("backup.title")');
	});

	it("derives an explicit tri-state view model with retry wired to the loader", () => {
		expect(backupSource).toContain("viewState");
		expect(backupSource).toContain('"loading"');
		expect(backupSource).toContain('"empty"');
		expect(backupSource).toContain("onRetry={load}");
	});

	it("keeps user behavior: snapshot create, restore preview and confirm", () => {
		expect(backupSource).toContain("createSnapshot(");
		expect(backupSource).toContain("previewRestore(");
		expect(backupSource).toContain("confirmRestore");
		expect(backupSource).toContain("summarizeSnapshotDiff");
	});

	it("drops the retired aura-era scaffolding", () => {
		expect(backupSource).not.toContain("aura-page");
		expect(backupSource).not.toContain("data-aura-echo");
		expect(backupSource).not.toContain("aura-kicker");
	});

	it("gates smooth scrolling behind both reduced-motion signals", () => {
		expect(backupSource).toContain("prefers-reduced-motion: reduce");
		expect(backupSource).toContain('[data-motion="reduce"]');
	});

	it("adds the new backup i18n keys in both dictionaries", () => {
		expect(i18nSource).toContain('"backup.title": "存档管理"');
		expect(i18nSource).toContain('"backup.title": "Save Backup"');
		expect(i18nSource).toContain('"backup.candidate_meta"');
		expect(i18nSource).toContain('"backup.restore_diff"');
		expect(i18nSource).toContain('"backup.confirm_restore"');
	});
});

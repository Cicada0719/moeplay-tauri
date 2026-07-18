import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";
import { getThemePack } from "../theme-packs";

const root = process.cwd();
const registrySource = readFileSync(resolve(root, "src/lib/theme-packs.ts"), "utf8");
const tokenSource = readFileSync(resolve(root, "src/lib/styles/themes/borderless-lumen.css"), "utf8");

describe("borderless-lumen theme pack contract", () => {
	it("registers the pack in the theme pack registry", () => {
		expect(registrySource).toContain('"borderless-lumen"');
		expect(registrySource).toContain("无界流光");
	});

	it("ships the calibrated token block", () => {
		expect(tokenSource).toContain('[data-theme-pack="borderless-lumen"]');
		expect(tokenSource).toContain("--accent: #7c5cff");
		expect(tokenSource).toContain("--mascot-accent: #56e0d4");
		expect(tokenSource).toContain("--theme-ambient: rgba(124, 92, 255, .22)");
	});

	it("ships the final theme-art composition module", () => {
		const artSource = readFileSync(resolve(root, "scripts/theme-art/borderless-lumen.mjs"), "utf8");
		expect(artSource).toContain("export const wallpapers");
		expect(artSource).toContain("export const preview");
		expect(artSource).toContain("export const mascot");
		expect(artSource).not.toContain("WAVE F PLACEHOLDER");
		for (const color of ["#7c5cff", "#56e0d4", "#e056a8"]) {
			expect(artSource).toContain(color);
		}
	});

	it("bundles all eight generated assets", () => {
		const dir = resolve(root, "src/lib/assets/themes/borderless-lumen");
		for (const name of [
			"wallpaper-1.jpg",
			"wallpaper-2.jpg",
			"wallpaper-3.jpg",
			"wallpaper-1-blur.jpg",
			"wallpaper-2-blur.jpg",
			"wallpaper-3-blur.jpg",
			"preview.jpg",
			"mascot.png",
		]) {
			expect(existsSync(resolve(dir, name)), name).toBe(true);
		}
	});

	it("uses a supported decoration", () => {
		expect(["petals", "light-particles", "digital-rain"]).toContain(getThemePack("borderless-lumen").decoration);
	});
});

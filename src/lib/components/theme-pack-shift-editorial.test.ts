import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { describe, expect, it } from "vitest";
import { getThemePack } from "../theme-packs";

const root = process.cwd();
const registrySource = readFileSync(resolve(root, "src/lib/theme-packs.ts"), "utf8");
const tokenSource = readFileSync(resolve(root, "src/lib/styles/themes/shift-editorial.css"), "utf8");

describe("shift-editorial theme pack contract", () => {
	it("registers the pack in the theme pack registry", () => {
		expect(registrySource).toContain('"shift-editorial"');
		expect(registrySource).toContain("素纸编集");
	});

	it("ships the calibrated token block", () => {
		expect(tokenSource).toContain('[data-theme-pack="shift-editorial"]');
		expect(tokenSource).toContain("--accent: #d4293c");
		expect(tokenSource).toContain("--text-primary: #18150f");
		expect(tokenSource).toContain("--bg-deep: #f7f4ee");
	});

	it("ships the final composition module per spec", async () => {
		const art = await import(pathToFileURL(resolve(root, "scripts/theme-art/shift-editorial.mjs")).href);
		expect(Array.isArray(art.wallpapers)).toBe(true);
		expect(art.wallpapers).toHaveLength(3);
		expect(typeof art.preview).toBe("string");
		expect(typeof art.mascot).toBe("string");
		// spec 构图要点:信号红 accent、wallpaper-1 的 MOE 大标题、preview 的「素纸编集」信息条
		expect(art.wallpapers[0]).toContain("#d4293c");
		expect(art.wallpapers[0]).toContain("MOE");
		expect(art.preview).toContain("素纸编集");
		expect(art.mascot).toContain("#d4293c");
		for (const html of [...art.wallpapers, art.preview, art.mascot]) {
			expect(html).not.toContain("WAVE F PLACEHOLDER");
		}
	});

	it("bundles all eight generated assets", () => {
		const dir = resolve(root, "src/lib/assets/themes/shift-editorial");
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
		expect(["petals", "light-particles", "digital-rain"]).toContain(getThemePack("shift-editorial").decoration);
	});
});

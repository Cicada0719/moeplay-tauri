import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { describe, expect, it } from "vitest";
import { getThemePack } from "../theme-packs";

const root = process.cwd();
const registrySource = readFileSync(resolve(root, "src/lib/theme-packs.ts"), "utf8");
const tokenSource = readFileSync(resolve(root, "src/lib/styles/themes/astral-rail.css"), "utf8");

describe("astral-rail theme pack contract", () => {
	it("registers the pack in the theme pack registry", () => {
		expect(registrySource).toContain('"astral-rail"');
		expect(registrySource).toContain("星穹旅人");
	});

	it("ships the calibrated token block", () => {
		expect(tokenSource).toContain('[data-theme-pack="astral-rail"]');
		expect(tokenSource).toContain("--accent: #d8b45a");
	});

	it("bundles all eight generated assets", () => {
		const dir = resolve(root, "src/lib/assets/themes/astral-rail");
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
		expect(["petals", "light-particles", "digital-rain"]).toContain(getThemePack("astral-rail").decoration);
	});

	it("ships the final wave-t compositions with the pipeline export shape", async () => {
		const art = await import(pathToFileURL(resolve(root, "scripts/theme-art/astral-rail.mjs")).href);
		expect(Array.isArray(art.wallpapers)).toBe(true);
		expect(art.wallpapers).toHaveLength(3);
		for (const html of [...art.wallpapers, art.preview, art.mascot]) {
			expect(typeof html).toBe("string");
			expect(html).not.toContain("WAVE F PLACEHOLDER");
		}
		expect(art.wallpapers.join("\n")).toContain("#d8b45a");
	});
});

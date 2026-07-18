import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { describe, expect, it } from "vitest";
import { getThemePack } from "../theme-packs";

const root = process.cwd();
const registrySource = readFileSync(resolve(root, "src/lib/theme-packs.ts"), "utf8");
const tokenSource = readFileSync(resolve(root, "src/lib/styles/themes/caution-industrial.css"), "utf8");
const artPath = resolve(root, "scripts/theme-art/caution-industrial.mjs");
const artSource = readFileSync(artPath, "utf8");

describe("caution-industrial theme pack contract", () => {
	it("registers the pack in the theme pack registry", () => {
		expect(registrySource).toContain('"caution-industrial"');
		expect(registrySource).toContain("警戒工业");
	});

	it("ships the calibrated token block", () => {
		expect(tokenSource).toContain('[data-theme-pack="caution-industrial"]');
		expect(tokenSource).toContain("--accent: #f59e0b");
		expect(tokenSource).toContain("--accent-hi: #ffb93d");
		expect(tokenSource).toContain("--mascot-accent: #9aa4b0");
	});

	it("ships the final composition module per spec", async () => {
		const art = (await import(pathToFileURL(artPath).href)) as {
			wallpapers: string[];
			preview: string;
			mascot: string;
		};
		expect(Array.isArray(art.wallpapers)).toBe(true);
		expect(art.wallpapers).toHaveLength(3);
		for (const html of [...art.wallpapers, art.preview, art.mascot]) {
			expect(typeof html).toBe("string");
			expect(html).toContain("<!DOCTYPE html>");
		}
		expect(artSource).not.toContain("WAVE F PLACEHOLDER");
		expect(artSource).toContain("#f59e0b");
		expect(artSource).toContain("#9aa4b0");
	});

	it("bundles all eight generated assets", () => {
		const dir = resolve(root, "src/lib/assets/themes/caution-industrial");
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
		expect(["petals", "light-particles", "digital-rain"]).toContain(getThemePack("caution-industrial").decoration);
	});
});

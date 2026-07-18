import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { describe, expect, it } from "vitest";
import { getThemePack } from "../theme-packs";

const root = process.cwd();
const registrySource = readFileSync(resolve(root, "src/lib/theme-packs.ts"), "utf8");
const tokenSource = readFileSync(resolve(root, "src/lib/styles/themes/phantom-pop.css"), "utf8");

describe("phantom-pop theme pack contract", () => {
	it("registers the pack in the theme pack registry", () => {
		expect(registrySource).toContain('"phantom-pop"');
		expect(registrySource).toContain("魅影波普");
	});

	it("ships the calibrated token block", () => {
		expect(tokenSource).toContain('[data-theme-pack="phantom-pop"]');
		expect(tokenSource).toContain("--accent: #e6242f");
		expect(tokenSource).toContain("--mascot-accent: #ffffff");
	});

	it("ships the refined art module matching the spec shape", async () => {
		const artSource = readFileSync(resolve(root, "scripts/theme-art/phantom-pop.mjs"), "utf8");
		expect(artSource).not.toContain("WAVE F PLACEHOLDER");
		expect(artSource).toContain("#e6242f");
		const art = await import(/* @vite-ignore */ pathToFileURL(resolve(root, "scripts/theme-art/phantom-pop.mjs")).href);
		expect(art.wallpapers).toHaveLength(3);
		for (const html of art.wallpapers) {
			expect(typeof html).toBe("string");
		}
		expect(typeof art.preview).toBe("string");
		expect(typeof art.mascot).toBe("string");
	});

	it("bundles all eight generated assets", () => {
		const dir = resolve(root, "src/lib/assets/themes/phantom-pop");
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
		expect(["petals", "light-particles", "digital-rain"]).toContain(getThemePack("phantom-pop").decoration);
	});
});

import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { cleanup, render, waitFor } from "@testing-library/svelte";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { parseCssColorToRgb } from "./model/adaptiveChroma";

const { loadPalette } = vi.hoisted(() => ({
  loadPalette: vi.fn(),
}));

vi.mock("./chroma/imagePalette", () => ({
  loadAdaptiveChromaPalette: loadPalette,
}));

import AdaptiveChromaStage from "./chroma/AdaptiveChromaStage.svelte";

const root = process.cwd();
const STYLE_FILES = [
  "src/lib/features/media-workspace/styles/game-visual.css",
  "src/lib/features/media-workspace/styles/game-scene.css",
  "src/lib/features/media-workspace/styles/media-workspace.css",
] as const;
const SHELL_FILE = "src/lib/features/media-workspace/shell/MediaWorkspaceShell.svelte";
const STAGE_FILE = "src/lib/features/media-workspace/chroma/AdaptiveChromaStage.svelte";

function readSource(path: string): string {
  return readFileSync(resolve(root, path), "utf8");
}

const styleContents = STYLE_FILES.map((path) => [path, readSource(path)] as const);
const shellSource = readSource(SHELL_FILE);
const stageSource = readSource(STAGE_FILE);

describe("media-workspace theme consistency", () => {
  describe("legacy hardcoded palette is gone from style sources", () => {
    const banned = [
      "--nd-paper: #",
      "--nd-paper-soft: #",
      "--nd-ink: #",
      "#e9e7e1",
      "#d7d5cf",
      "rgb(10 11 13",
      "--director-paper: #",
      "--director-signal: #",
      "#eeeae0",
      "#c7472f",
      "199 71 47",
      "232 85 127",
      "#0b0d10",
      "#0b0b0d",
      "#08090b",
      "#f0efe9",
      "rgb(240 239 233 / .38)",
      "rgb(240 239 233 / .16)",
      "rgb(8 10 13",
      "rgb(5 7 10",
      "rgb(4 6 9",
      "#050505",
    ];
    for (const [path, content] of styleContents) {
      it(`${path} contains no hardcoded paper/ink/signal colors`, () => {
        for (const token of banned) {
          expect(content, `expected ${path} to not contain ${JSON.stringify(token)}`).not.toContain(token);
        }
      });
    }

    it("MediaWorkspaceShell.svelte drops the salmon fallbacks and dark glass", () => {
      for (const token of ["199 71 47", "rgb(255 255 255", "#c7472f", "--c-accent", "rgb(8 10 13", "13 16 20", "10 12 16", "70 76 88"]) {
        expect(shellSource, `expected shell to not contain ${JSON.stringify(token)}`).not.toContain(token);
      }
    });
  });

  describe("semantic token mapping is in place", () => {
    it("game-visual.css maps the nd-* palette onto theme tokens", () => {
      const content = styleContents[0][1];
      expect(content).toContain("--nd-paper: var(--bg-elev)");
      expect(content).toContain("--nd-paper-soft: var(--bg-card)");
      expect(content).toContain("--nd-ink: var(--text-primary)");
      expect(content).toContain("--nd-line: var(--border-hover)");
      expect(content).toContain("var(--media-accent, var(--accent))");
      expect(content).toContain("var(--media-on-accent, var(--bg-deep))");
    });

    it("game-scene.css maps the fs-* stage onto theme tokens", () => {
      const content = styleContents[1][1];
      expect(content).toContain("background:var(--bg-deep)");
      expect(content).toContain("color:var(--text-primary)");
      expect(content).toContain("var(--media-accent, var(--accent))");
      expect(content).toContain("var(--media-on-accent, var(--bg-deep))");
    });

    it("media-workspace.css maps director tokens and v2 accents onto theme tokens", () => {
      const content = styleContents[2][1];
      expect(content).toContain("--director-paper: var(--bg-elev)");
      expect(content).toContain("--director-signal: var(--accent)");
      expect(content).toContain("background-color: var(--bg-void)");
      expect(content).toContain("--v2-accent: var(--media-accent, var(--accent))");
      expect(content).toContain("color-mix(in srgb, var(--accent) 7.5%, transparent)");
    });

    it("MediaWorkspaceShell.svelte consumes semantic tokens", () => {
      expect(shellSource).toContain("color: var(--accent)");
      expect(shellSource).toContain("color-mix(in srgb, var(--bg-deep) 74%, transparent)");
      expect(shellSource).toContain("border-top: 1px solid var(--border)");
    });
  });

  describe("reduced-motion dual signaling", () => {
    for (const [path, content] of styleContents) {
      it(`${path} honors both prefers-reduced-motion and data-motion`, () => {
        expect(content).toContain("prefers-reduced-motion");
        expect(content).toContain('[data-motion="reduce"]');
      });
    }
  });

  describe("parseCssColorToRgb", () => {
    it("parses hex colors", () => {
      expect(parseCssColorToRgb("#d4293c")).toEqual({ r: 212, g: 41, b: 60 });
      expect(parseCssColorToRgb("#fff")).toEqual({ r: 255, g: 255, b: 255 });
      expect(parseCssColorToRgb(" #E8557F ")).toEqual({ r: 232, g: 85, b: 127 });
    });

    it("parses rgb()/rgba() functional colors", () => {
      expect(parseCssColorToRgb("rgb(199, 71, 47)")).toEqual({ r: 199, g: 71, b: 47 });
      expect(parseCssColorToRgb("rgba(232, 85, 127, .16)")).toEqual({ r: 232, g: 85, b: 127 });
      expect(parseCssColorToRgb("rgb(124 92 255 / .5)")).toEqual({ r: 124, g: 92, b: 255 });
    });

    it("rejects unparsable values", () => {
      expect(parseCssColorToRgb("")).toBeNull();
      expect(parseCssColorToRgb("var(--accent)")).toBeNull();
      expect(parseCssColorToRgb("transparent")).toBeNull();
    });
  });

  describe("--media-accent-rgb theme fallback", () => {
    it("AdaptiveChromaStage resolves the theme accent token at runtime", () => {
      expect(stageSource).toContain('getPropertyValue("--accent")');
      expect(stageSource).toContain("getComputedStyle");
    });

    it("falls back to the current theme accent when no cover palette is available", () => {
      document.documentElement.style.setProperty("--accent", "#d4293c");
      const { container } = render(AdaptiveChromaStage, { props: {} });
      const stage = container.firstElementChild as HTMLElement;
      expect(stage.style.getPropertyValue("--media-accent-rgb")).toBe("212 41 60");
      expect(stage.style.getPropertyValue("--media-accent")).toBe("rgb(212 41 60)");
    });

    it("re-reads the theme accent when the theme pack attribute changes", async () => {
      document.documentElement.style.setProperty("--accent", "#d4293c");
      document.documentElement.setAttribute("data-theme-pack", "shift-editorial");
      const { container } = render(AdaptiveChromaStage, { props: {} });
      const stage = container.firstElementChild as HTMLElement;
      expect(stage.style.getPropertyValue("--media-accent-rgb")).toBe("212 41 60");

      document.documentElement.style.setProperty("--accent", "#7c5cff");
      document.documentElement.setAttribute("data-theme-pack", "borderless-lumen");
      await waitFor(() => expect(stage.style.getPropertyValue("--media-accent-rgb")).toBe("124 92 255"));
    });

    it("keeps an explicit themePalette prop authoritative over the theme token", () => {
      document.documentElement.style.setProperty("--accent", "#d4293c");
      const { container } = render(AdaptiveChromaStage, {
        props: {
          themePalette: {
            primary: { r: 10, g: 20, b: 30 },
            secondary: { r: 40, g: 50, b: 60 },
            accent: { r: 1, g: 2, b: 3 },
            surface: { r: 15, g: 17, b: 22 },
            foreground: { r: 248, g: 247, b: 244 },
            isDark: true,
            source: "fallback",
          },
        },
      });
      const stage = container.firstElementChild as HTMLElement;
      expect(stage.style.getPropertyValue("--media-accent-rgb")).toBe("1 2 3");
    });
  });
});

beforeEach(() => {
  loadPalette.mockReset();
});

afterEach(() => {
  cleanup();
  document.documentElement.style.removeProperty("--accent");
  document.documentElement.removeAttribute("data-theme-pack");
});

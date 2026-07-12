import { cleanup, render, waitFor } from "@testing-library/svelte";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { AdaptiveChromaPalette } from "../model/chromaTypes";

const { loadPalette } = vi.hoisted(() => ({
  loadPalette: vi.fn(),
}));

vi.mock("./imagePalette", () => ({
  loadAdaptiveChromaPalette: loadPalette,
}));

import AdaptiveChromaStage from "./AdaptiveChromaStage.svelte";

const FIRST_PALETTE: AdaptiveChromaPalette = {
  primary: { r: 220, g: 40, b: 70 },
  secondary: { r: 40, g: 80, b: 160 },
  accent: { r: 240, g: 100, b: 130 },
  surface: { r: 16, g: 18, b: 24 },
  foreground: { r: 248, g: 247, b: 244 },
  isDark: true,
  source: "media",
};

const SECOND_PALETTE: AdaptiveChromaPalette = {
  ...FIRST_PALETTE,
  primary: { r: 30, g: 180, b: 210 },
  accent: { r: 60, g: 210, b: 230 },
};

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((resolvePromise, rejectPromise) => {
    resolve = resolvePromise;
    reject = rejectPromise;
  });
  return { promise, resolve, reject };
}

function installMatchMedia(matches: (query: string) => boolean) {
  Object.defineProperty(window, "matchMedia", {
    configurable: true,
    value: vi.fn((query: string) => ({
      matches: matches(query),
      media: query,
      onchange: null,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      addListener: vi.fn(),
      removeListener: vi.fn(),
      dispatchEvent: vi.fn(),
    })),
  });
}

beforeEach(() => {
  loadPalette.mockReset();
  installMatchMedia(() => false);
});

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

describe("AdaptiveChromaStage", () => {
  it("uses the exact normalized source for both the background and palette extraction", async () => {
    loadPalette.mockResolvedValue(FIRST_PALETTE);
    const source = "  https://cdn.example.test/art/cover%201.jpg?rev=4  ";
    const { container } = render(AdaptiveChromaStage, { props: { src: source } });
    const stage = container.firstElementChild as HTMLElement;

    expect(loadPalette).toHaveBeenCalledWith(source.trim());
    expect(stage.style.getPropertyValue("--adaptive-chroma-source")).toContain(source.trim());
    expect(stage.style.getPropertyValue("--adaptive-chroma-background-image")).toContain(source.trim());
    expect(stage.style.backgroundImage).toBe("var(--adaptive-chroma-background-image)");

    await waitFor(() => expect(stage.dataset.adaptiveChromaState).toBe("ready"));
    expect(stage.style.getPropertyValue("--adaptive-chroma-primary-rgb")).toBe(
      stage.style.getPropertyValue("--media-primary-rgb"),
    );
  });

  it("ignores a stale extraction when src changes", async () => {
    const first = deferred<AdaptiveChromaPalette>();
    const second = deferred<AdaptiveChromaPalette>();
    loadPalette.mockImplementation((url: string) => url.endsWith("first.jpg") ? first.promise : second.promise);

    const view = render(AdaptiveChromaStage, { props: { src: "https://example.test/first.jpg", strength: "immersive" } });
    const stage = view.container.firstElementChild as HTMLElement;
    await view.rerender({ src: "https://example.test/second.jpg", strength: "immersive" });

    second.resolve(SECOND_PALETTE);
    await waitFor(() => expect(stage.dataset.adaptiveChromaState).toBe("ready"));
    const secondPrimary = stage.style.getPropertyValue("--adaptive-chroma-primary-rgb");

    first.resolve(FIRST_PALETTE);
    await Promise.resolve();
    await Promise.resolve();
    expect(stage.style.getPropertyValue("--adaptive-chroma-primary-rgb")).toBe(secondPrimary);
    expect(stage.style.getPropertyValue("--adaptive-chroma-source")).toContain("second.jpg");
  });

  it("disables decorative imagery and transitions for contrast and reduced-motion preferences", async () => {
    installMatchMedia((query) => query.includes("prefers-reduced-motion") || query.includes("prefers-contrast"));
    loadPalette.mockResolvedValue(FIRST_PALETTE);

    const { container } = render(AdaptiveChromaStage, { props: { src: "https://example.test/cover.jpg" } });
    const stage = container.firstElementChild as HTMLElement;

    await waitFor(() => expect(stage.dataset.adaptiveChromaContrast).toBe("high"));
    expect(stage.dataset.adaptiveChromaReducedMotion).toBe("true");
    expect(stage.style.getPropertyValue("--adaptive-chroma-background-image")).toBe("none");
    expect(stage.style.getPropertyValue("--adaptive-chroma-transition-duration")).toBe("0ms");
    expect(stage.style.getPropertyValue("--adaptive-chroma-source")).toContain("cover.jpg");
  });
});

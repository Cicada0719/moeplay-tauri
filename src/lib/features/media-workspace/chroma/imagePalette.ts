import {
  extractPaletteFromPixels,
  fallbackPalette,
  type PalettePixelSource,
} from "../model/adaptiveChroma";
import type { AdaptiveChromaPalette } from "../model/chromaTypes";
import {
  adaptiveChromaPaletteCache,
  type AdaptiveChromaPaletteCache,
} from "./paletteCache";

export const MAX_CHROMA_SAMPLE_DIMENSION = 32;
const IMAGE_LOAD_TIMEOUT_MS = 15_000;

export interface SampleSize {
  width: number;
  height: number;
}

export function calculateSampleSize(
  sourceWidth: number,
  sourceHeight: number,
  maxDimension = MAX_CHROMA_SAMPLE_DIMENSION,
): SampleSize {
  const width = Math.max(1, Math.floor(sourceWidth));
  const height = Math.max(1, Math.floor(sourceHeight));
  const limit = Math.max(1, Math.floor(maxDimension));
  const scale = Math.min(1, limit / Math.max(width, height));
  return {
    width: Math.max(1, Math.round(width * scale)),
    height: Math.max(1, Math.round(height * scale)),
  };
}

function loadImage(url: string): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    if (typeof Image === "undefined") {
      reject(new Error("Adaptive Chroma requires a browser image implementation."));
      return;
    }

    const image = new Image();
    let settled = false;
    const timeout = globalThis.setTimeout(() => finish(new Error("Adaptive Chroma image load timed out.")), IMAGE_LOAD_TIMEOUT_MS);

    function cleanup() {
      globalThis.clearTimeout(timeout);
      image.onload = null;
      image.onerror = null;
    }

    function finish(error?: Error) {
      if (settled) return;
      settled = true;
      cleanup();
      if (error) reject(error);
      else resolve(image);
    }

    image.decoding = "async";
    if (/^https?:\/\//i.test(url)) image.crossOrigin = "anonymous";
    image.onload = () => finish();
    image.onerror = () => finish(new Error("Adaptive Chroma could not load the image."));

    try {
      image.src = url;
    } catch {
      finish(new Error("Adaptive Chroma received an unsupported image URL."));
    }
  });
}

export async function sampleImagePixels(url: string): Promise<PalettePixelSource> {
  if (!url.trim()) throw new Error("Adaptive Chroma requires a non-empty image URL.");
  if (typeof document === "undefined") throw new Error("Adaptive Chroma canvas is unavailable outside the browser.");

  const image = await loadImage(url);
  const sourceWidth = image.naturalWidth || image.width;
  const sourceHeight = image.naturalHeight || image.height;
  if (!sourceWidth || !sourceHeight) throw new Error("Adaptive Chroma image has no drawable dimensions.");

  const size = calculateSampleSize(sourceWidth, sourceHeight);
  const canvas = document.createElement("canvas");
  canvas.width = size.width;
  canvas.height = size.height;
  const context = canvas.getContext("2d", { willReadFrequently: true });
  if (!context) throw new Error("Adaptive Chroma could not create a 2D canvas context.");

  try {
    context.clearRect(0, 0, size.width, size.height);
    context.drawImage(image, 0, 0, size.width, size.height);
    const imageData = context.getImageData(0, 0, size.width, size.height);
    return { data: imageData.data, width: size.width, height: size.height };
  } catch {
    throw new Error("Adaptive Chroma could not read image pixels (possibly due to cross-origin restrictions).");
  }
}

export async function resolvePaletteWithCache(
  url: string,
  extract: () => Promise<AdaptiveChromaPalette>,
  cache: AdaptiveChromaPaletteCache = adaptiveChromaPaletteCache,
): Promise<AdaptiveChromaPalette> {
  const cached = cache.get(url);
  if (cached) return cached;

  const palette = await extract();
  // A fallback result is useful to the caller, but does not represent a successful extraction.
  if (palette.source === "media") cache.set(url, palette);
  return palette;
}

const inFlight = new Map<string, Promise<AdaptiveChromaPalette>>();

export function loadAdaptiveChromaPalette(url: string): Promise<AdaptiveChromaPalette> {
  const normalizedUrl = url.trim();
  if (!normalizedUrl) return Promise.resolve(fallbackPalette());

  const pending = inFlight.get(normalizedUrl);
  if (pending) return pending;

  const request = resolvePaletteWithCache(normalizedUrl, async () => {
    const pixels = await sampleImagePixels(normalizedUrl);
    return extractPaletteFromPixels(pixels);
  }).finally(() => {
    if (inFlight.get(normalizedUrl) === request) inFlight.delete(normalizedUrl);
  });

  inFlight.set(normalizedUrl, request);
  return request;
}

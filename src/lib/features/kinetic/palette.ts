/**
 * 舞台配色：读取 html[data-theme-pack] 语义 token 并解析为 0..1 RGB。
 * 仅消费 --bg-deep/--bg-elev/--accent 等 token，不硬编码主题色；
 * token 缺失或无法解析时使用中性兜底色。
 */

import type { KineticPalette, KineticRgb } from "./types";

const FALLBACK_BG: KineticRgb = { r: 0.02, g: 0.027, b: 0.04 };
const FALLBACK_SURFACE: KineticRgb = { r: 0.125, g: 0.15, b: 0.2 };
const FALLBACK_ACCENT: KineticRgb = { r: 0.91, g: 0.33, b: 0.5 };

export function parseCssColorToRgb(value: string | null | undefined): KineticRgb | null {
  if (!value) return null;
  const input = value.trim();
  if (!input) return null;

  const hex = /^#([0-9a-f]{3}|[0-9a-f]{6}|[0-9a-f]{8})$/i.exec(input);
  if (hex) {
    let body = hex[1];
    if (body.length === 3) body = body.split("").map((ch) => ch + ch).join("");
    if (body.length === 8) body = body.slice(0, 6);
    const r = parseInt(body.slice(0, 2), 16);
    const g = parseInt(body.slice(2, 4), 16);
    const b = parseInt(body.slice(4, 6), 16);
    if ([r, g, b].some((channel) => Number.isNaN(channel))) return null;
    return { r: r / 255, g: g / 255, b: b / 255 };
  }

  const fn = /^rgba?\(\s*([^)]+)\)$/i.exec(input);
  if (fn) {
    const parts = fn[1].split(/[\s,/]+/).filter(Boolean);
    if (parts.length < 3) return null;
    const channels = parts.slice(0, 3).map((part) => {
      if (part.endsWith("%")) return (parseFloat(part) / 100) * 255;
      return parseFloat(part);
    });
    if (channels.some((channel) => Number.isNaN(channel))) return null;
    return { r: channels[0] / 255, g: channels[1] / 255, b: channels[2] / 255 };
  }

  return null;
}

function lerpTowardWhite(color: KineticRgb, amount: number): KineticRgb {
  return {
    r: color.r + (1 - color.r) * amount,
    g: color.g + (1 - color.g) * amount,
    b: color.b + (1 - color.b) * amount,
  };
}

function readToken(name: string): string {
  if (typeof window === "undefined" || typeof document === "undefined") return "";
  try {
    return window.getComputedStyle(document.documentElement).getPropertyValue(name).trim();
  } catch {
    return "";
  }
}

export function readKineticPalette(): KineticPalette {
  const accent = parseCssColorToRgb(readToken("--accent")) ?? FALLBACK_ACCENT;
  return {
    bg: parseCssColorToRgb(readToken("--bg-deep")) ?? parseCssColorToRgb(readToken("--bg-base")) ?? FALLBACK_BG,
    surface: parseCssColorToRgb(readToken("--bg-elev")) ?? FALLBACK_SURFACE,
    accent,
    glow: lerpTowardWhite(accent, 0.35),
  };
}

/**
 * 监听主题切换（data-theme-pack / data-color-mode / data-theme 变化），
 * 主题 token 变化时以最新 palette 触发回调。返回取消订阅函数。
 */
export function watchKineticPalette(listener: (palette: KineticPalette) => void): () => void {
  if (typeof MutationObserver !== "function" || typeof document === "undefined") {
    return () => undefined;
  }
  const sync = () => listener(readKineticPalette());
  const observer = new MutationObserver(sync);
  observer.observe(document.documentElement, {
    attributes: true,
    attributeFilter: ["data-theme-pack", "data-color-mode", "data-theme"],
  });
  return () => observer.disconnect();
}

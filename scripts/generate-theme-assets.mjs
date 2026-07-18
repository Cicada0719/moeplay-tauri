#!/usr/bin/env node
// Wave F 主题占位资产生成管线:用 Playwright 渲染 scripts/theme-art/<id>.mjs 的自包含构图,
// 为每个新主题包产出 8 件资产(wallpaper-{1,2,3}.jpg / *-blur.jpg / preview.jpg / mascot.png)。
//
// 用法:
//   node scripts/generate-theme-assets.mjs                 # 生成全部新包(5 × 8 = 40 件)
//   node scripts/generate-theme-assets.mjs --pack=<id>     # 只重生成单包,如 --pack=phantom-pop

import { mkdir, stat } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");

const PACK_IDS = ["shift-editorial", "phantom-pop", "caution-industrial", "astral-rail", "borderless-lumen"];

const packArg = process.argv.find((arg) => arg.startsWith("--pack="));
const only = packArg ? packArg.slice("--pack=".length) : null;
if (only && !PACK_IDS.includes(only)) {
  console.error(`[theme-assets] unknown pack "${only}"; expected one of: ${PACK_IDS.join(", ")}`);
  process.exit(1);
}
const targets = only ? [only] : PACK_IDS;

async function render(browser, html, out, { width, height, type, quality, omitBackground = false }) {
  const page = await browser.newPage({ viewport: { width, height } });
  await page.setContent(html);
  await page.screenshot({
    path: out,
    type,
    ...(quality == null ? {} : { quality }),
    ...(omitBackground ? { omitBackground: true } : {}),
  });
  await page.close();
  const { size } = await stat(out);
  console.log(`[theme-assets] ${out.slice(root.length + 1)} (${(size / 1024).toFixed(1)} KB)`);
}

async function generatePack(browser, id) {
  const art = await import(`./theme-art/${id}.mjs`);
  if (!Array.isArray(art.wallpapers) || art.wallpapers.length !== 3 || typeof art.preview !== "string" || typeof art.mascot !== "string") {
    throw new Error(`scripts/theme-art/${id}.mjs must export { wallpapers: [html x3], preview, mascot }`);
  }

  const outDir = resolve(root, "src/lib/assets/themes", id);
  await mkdir(outDir, { recursive: true });

  for (let i = 0; i < 3; i += 1) {
    await render(browser, art.wallpapers[i], resolve(outDir, `wallpaper-${i + 1}.jpg`), {
      width: 1920, height: 1080, type: "jpeg", quality: 82,
    });
    // blur 占位:同构图在 32×18 小视口重渲染
    await render(browser, art.wallpapers[i], resolve(outDir, `wallpaper-${i + 1}-blur.jpg`), {
      width: 32, height: 18, type: "jpeg", quality: 70,
    });
  }
  await render(browser, art.preview, resolve(outDir, "preview.jpg"), {
    width: 800, height: 500, type: "jpeg", quality: 82,
  });
  await render(browser, art.mascot, resolve(outDir, "mascot.png"), {
    width: 512, height: 512, type: "png", omitBackground: true,
  });
}

const browser = await chromium.launch();
let generated = 0;
try {
  for (const id of targets) {
    console.log(`[theme-assets] pack ${id}`);
    await generatePack(browser, id);
    generated += 8;
  }
} finally {
  await browser.close();
}
console.log(`[theme-assets] done: ${generated} assets across ${targets.length} pack(s)`);

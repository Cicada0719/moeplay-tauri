// Theme artwork source used by scripts/generate-theme-assets.mjs.
// 色彩语言:深空靛紫 #090a1a + 星轨金线 #d8b45a + 银河星光 #8ea2ff。
// 自包含 HTML:内联 <style>/内联 SVG、零外部资源、仅系统字体栈、无脚本。
// 壁纸统一 viewBox 1920×1080,管线在 32×18 视口重渲染同一构图得到 blur 占位。

const SPACE = "#090a1a";
const SPACE_HI = "#141a3a";
const GOLD = "#d8b45a";
const GOLD_HI = "#eed390";
const STAR_BLUE = "#8ea2ff";
const PLANET_LO = "#4a5aa8";

function page(body, bg = SPACE) {
  return `<!DOCTYPE html>
<html><head><meta charset="utf-8"><style>
html,body{margin:0;padding:0;width:100%;height:100%;overflow:hidden;background:${bg}}
body{position:relative}
svg{display:block}
</style></head><body>${body}</body></html>`;
}

// mulberry32 确定性伪随机:同一 seed 每次生成同一星场,产物可复现。
function mulberry32(seed) {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

function svgLayer(viewBox, inner) {
  return `<svg viewBox="${viewBox}" preserveAspectRatio="xMidYMid slice" style="position:absolute;inset:0;width:100%;height:100%" xmlns="http://www.w3.org/2000/svg">${inner}</svg>`;
}

// 极低透明度噪点粒层:对大面积平滑渐变做抖动,抑制 JPEG banding。
const GRAIN_DEF = `<filter id="grain" x="0" y="0" width="100%" height="100%"><feTurbulence type="fractalNoise" baseFrequency="0.8" numOctaves="2" stitchTiles="stitch"/><feColorMatrix type="matrix" values="0 0 0 0 1 0 0 0 0 1 0 0 0 0 1 0 0 0 0.045 0"/></filter>`;
const GRAIN_RECT = `<rect x="0" y="0" width="1920" height="1080" filter="url(#grain)"/>`;

// 星尘粒子场:白/淡金小点(视觉直径约 1-3px),约 1/4 带 4px 光晕。
function starField(seed, count, w, h, { maxY = h, minOpacity = 0.35 } = {}) {
  const rand = mulberry32(seed);
  let out = "";
  for (let i = 0; i < count; i += 1) {
    const x = (rand() * w).toFixed(1);
    const y = (rand() * maxY).toFixed(1);
    const r = (0.5 + rand() * 1.1).toFixed(2);
    const fill = rand() < 0.35 ? GOLD_HI : "#ffffff";
    const opacity = (minOpacity + rand() * (0.95 - minOpacity)).toFixed(2);
    if (rand() < 0.25) {
      out += `<circle cx="${x}" cy="${y}" r="4" fill="${fill}" opacity="${(opacity * 0.22).toFixed(2)}"/>`;
    }
    out += `<circle cx="${x}" cy="${y}" r="${r}" fill="${fill}" opacity="${opacity}"/>`;
  }
  return out;
}

// ── wallpaper-1「银河铁道」────────────────────────────────────────────
// 深空底 + 两道金色轨道弧线自左下向右上掠过(2px/.8、1px/.4,大半径圆弧)
// + 星尘粒子场 + 右上远方淡紫行星(radial-gradient 球体)。
const wallpaper1 = page(`
<div style="position:absolute;inset:0;background:linear-gradient(165deg,${SPACE} 0%,#0c0f2b 55%,${SPACE_HI} 100%)"></div>
${svgLayer("0 0 1920 1080", `
<defs>
<radialGradient id="w1planet" cx="35%" cy="35%" r="85%">
<stop offset="0%" stop-color="#c2cdff"/>
<stop offset="42%" stop-color="${STAR_BLUE}"/>
<stop offset="100%" stop-color="${PLANET_LO}"/>
</radialGradient>
<radialGradient id="w1glow" cx="50%" cy="50%" r="50%">
<stop offset="0%" stop-color="${STAR_BLUE}" stop-opacity=".30"/>
<stop offset="100%" stop-color="${STAR_BLUE}" stop-opacity="0"/>
</radialGradient>
${GRAIN_DEF}
</defs>
${starField(4101, 90, 1920, 1080)}
<path d="M -150 1052 Q 800 402 2050 -178" fill="none" stroke="${GOLD}" stroke-width="1" stroke-opacity=".4" stroke-linecap="round"/>
<path d="M -150 1150 Q 800 500 2050 -80" fill="none" stroke="${GOLD}" stroke-width="2" stroke-opacity=".8" stroke-linecap="round"/>
<circle cx="1620" cy="228" r="150" fill="url(#w1glow)"/>
<circle cx="1620" cy="228" r="88" fill="url(#w1planet)"/>
${GRAIN_RECT}`)}
`);

// ── wallpaper-2「星图」──────────────────────────────────────────────
// 深空底 + 中央淡紫星云(opacity .2) + 9 星点金色折线星座
// + 两个同心虚线轨道圆环(rgba(216,180,90,.3),dasharray 4 6) + 角落等宽坐标注记。
const CONSTELLATION = [
  [420, 770], [556, 648], [694, 694], [828, 548], [962, 596],
  [1102, 436], [1238, 474], [1362, 326], [1490, 368],
];
const RING_HUB = CONSTELLATION[5];

function starMapMarks() {
  const points = CONSTELLATION.map(([x, y]) => `${x},${y}`).join(" ");
  let out = `<polyline points="${points}" fill="none" stroke="${GOLD}" stroke-width="1" stroke-opacity=".55" stroke-linejoin="round"/>`;
  for (const [x, y] of CONSTELLATION) {
    const isHub = x === RING_HUB[0] && y === RING_HUB[1];
    const r = isHub ? 3.2 : 2.3;
    const fill = isHub ? GOLD_HI : "#ffffff";
    out += `<circle cx="${x}" cy="${y}" r="${isHub ? 7 : 5}" fill="${fill}" opacity="${isHub ? 0.3 : 0.18}"/>`;
    out += `<circle cx="${x}" cy="${y}" r="${r}" fill="${fill}" opacity=".95"/>`;
  }
  return out;
}

const COORD_FONT = "Consolas, 'Courier New', monospace";
const wallpaper2 = page(`
<div style="position:absolute;inset:0;background:linear-gradient(150deg,${SPACE} 0%,#0b0e28 60%,#121636 100%)"></div>
${svgLayer("0 0 1920 1080", `
<defs>
<radialGradient id="w2nebula" cx="50%" cy="50%" r="50%">
<stop offset="0%" stop-color="${STAR_BLUE}" stop-opacity=".2"/>
<stop offset="100%" stop-color="${STAR_BLUE}" stop-opacity="0"/>
</radialGradient>
${GRAIN_DEF}
</defs>
${starField(5202, 55, 1920, 1080)}
<ellipse cx="960" cy="540" rx="720" ry="430" fill="url(#w2nebula)"/>
<circle cx="${RING_HUB[0]}" cy="${RING_HUB[1]}" r="140" fill="none" stroke="${GOLD}" stroke-opacity=".3" stroke-width="1" stroke-dasharray="4 6"/>
<circle cx="${RING_HUB[0]}" cy="${RING_HUB[1]}" r="220" fill="none" stroke="${GOLD}" stroke-opacity=".3" stroke-width="1" stroke-dasharray="4 6"/>
${starMapMarks()}
<text x="48" y="66" font-family="${COORD_FONT}" font-size="18" letter-spacing="2" fill="#ffffff" fill-opacity=".35">RA 05h 35m 17s</text>
<text x="48" y="94" font-family="${COORD_FONT}" font-size="18" letter-spacing="2" fill="#ffffff" fill-opacity=".35">DEC -05° 23′ 28″</text>
<text x="1872" y="1018" text-anchor="end" font-family="${COORD_FONT}" font-size="18" letter-spacing="2" fill="#ffffff" fill-opacity=".35">EPOCH J2000.0 · SOL 0449</text>
${GRAIN_RECT}`)}
`);

// ── wallpaper-3「晨曦跃迁」───────────────────────────────────────────
// 上方深空渐变(#090a1a → #141a3a) + 底部地平线辉光(#d8b45a → transparent,
// 大半径 radial-gradient,约 35% 高) + 16 条 60° 上升星轨(opacity 随高度衰减)。
function risingTrails(seed, count) {
  const rand = mulberry32(seed);
  let out = "";
  for (let i = 0; i < count; i += 1) {
    const x = 60 + rand() * 1800;
    const y = 240 + rand() * 800;
    const len = 55 + rand() * 125;
    const width = rand() < 0.5 ? 1 : 2;
    const color = rand() < 0.45 ? GOLD : "#ffffff";
    const opacity = 0.18 + 0.72 * (y / 1080);
    const x2 = (x + len * 0.5).toFixed(1);
    const y2 = (y - len * 0.8660254).toFixed(1);
    out += `<line x1="${x.toFixed(1)}" y1="${y.toFixed(1)}" x2="${x2}" y2="${y2}" stroke="${color}" stroke-width="${width}" stroke-opacity="${opacity.toFixed(2)}" stroke-linecap="round"/>`;
    out += `<circle cx="${x2}" cy="${y2}" r="${(width * 1.3).toFixed(1)}" fill="${color}" opacity="${Math.min(1, opacity + 0.2).toFixed(2)}"/>`;
  }
  return out;
}

const wallpaper3 = page(`
<div style="position:absolute;inset:0;background:linear-gradient(180deg,${SPACE} 0%,#0e1230 55%,${SPACE_HI} 100%)"></div>
${svgLayer("0 0 1920 1080", `
<defs>
<radialGradient id="w3horizon" cx="50%" cy="50%" r="50%">
<stop offset="0%" stop-color="${GOLD}" stop-opacity=".9"/>
<stop offset="38%" stop-color="${GOLD}" stop-opacity=".4"/>
<stop offset="70%" stop-color="${GOLD}" stop-opacity=".12"/>
<stop offset="100%" stop-color="${GOLD}" stop-opacity="0"/>
</radialGradient>
<radialGradient id="w3core" cx="50%" cy="50%" r="50%">
<stop offset="0%" stop-color="${GOLD_HI}" stop-opacity=".5"/>
<stop offset="100%" stop-color="${GOLD_HI}" stop-opacity="0"/>
</radialGradient>
${GRAIN_DEF}
</defs>
${starField(6303, 45, 1920, 1080, { maxY: 700 })}
<ellipse cx="960" cy="1200" rx="1550" ry="500" fill="url(#w3horizon)"/>
<ellipse cx="960" cy="1180" rx="820" ry="230" fill="url(#w3core)"/>
${risingTrails(6303, 16)}
${GRAIN_RECT}`)}
`);

export const wallpapers = [wallpaper1, wallpaper2, wallpaper3];

// ── preview(800×500)─────────────────────────────────────────────────
// 金色轨道圆环(带一颗环上卫星) + 中央「星穹旅人」衬线小标题(Georgia/serif,金)
// + 三颗星点(标题上方弧形排布)。
export const preview = page(`
<div style="position:absolute;inset:0;background:linear-gradient(160deg,${SPACE} 0%,#0d1132 60%,${SPACE_HI} 100%)"></div>
${svgLayer("0 0 800 500", `
${starField(7701, 40, 800, 500)}
<circle cx="400" cy="250" r="150" fill="none" stroke="${GOLD}" stroke-width="2" stroke-opacity=".8"/>
<circle cx="266" cy="182" r="9" fill="${GOLD_HI}" opacity=".3"/>
<circle cx="266" cy="182" r="4.5" fill="${GOLD_HI}"/>
<circle cx="322" cy="158" r="4" fill="#ffffff" opacity=".25"/>
<circle cx="322" cy="158" r="2.2" fill="#ffffff" opacity=".9"/>
<circle cx="400" cy="136" r="4" fill="${GOLD_HI}" opacity=".3"/>
<circle cx="400" cy="136" r="2.4" fill="${GOLD_HI}" opacity=".95"/>
<circle cx="478" cy="158" r="4" fill="#ffffff" opacity=".25"/>
<circle cx="478" cy="158" r="2.2" fill="#ffffff" opacity=".9"/>`)}
<div style="position:absolute;inset:0;display:flex;align-items:center;justify-content:center">
<span style="font-family:Georgia,'Times New Roman',serif;font-size:44px;letter-spacing:.28em;margin-left:.28em;color:${GOLD_HI};text-shadow:0 0 22px rgba(216,180,90,.45);white-space:nowrap">星穹旅人</span>
</div>`);

// ── mascot(512×512 透明 PNG)─────────────────────────────────────────
// 金色圆环行星:椭圆描边 6px 环穿过球体(后半环 → 球体 → 前半弧),
// 球体 #8ea2ff → #4a5aa8 径向渐变;右上一颗自绘 4 角金色小星。扁平优雅。
export const mascot = page(`
${svgLayer("0 0 512 512", `
<defs>
<radialGradient id="mPlanet" cx="35%" cy="35%" r="80%">
<stop offset="0%" stop-color="${STAR_BLUE}"/>
<stop offset="55%" stop-color="#6f82d8"/>
<stop offset="100%" stop-color="${PLANET_LO}"/>
</radialGradient>
</defs>
<g transform="rotate(-18 256 256)">
<ellipse cx="256" cy="256" rx="196" ry="62" fill="none" stroke="${GOLD}" stroke-width="6"/>
</g>
<circle cx="256" cy="256" r="118" fill="url(#mPlanet)"/>
<g transform="rotate(-18 256 256)">
<path d="M 60 256 A 196 62 0 0 0 452 256" fill="none" stroke="${GOLD}" stroke-width="6" stroke-linecap="round"/>
</g>
<polygon points="392,82 400,110 428,118 400,126 392,154 384,126 356,118 384,110" fill="${GOLD_HI}"/>`)}
`, "transparent");

// caution-industrial「警戒工业」终稿构图:枪灰金属底 + 工程蓝图网格 + 警示条纹 + HUD 刻度。
// 设计依据 plans/theme-specs/caution-industrial.md;色值严格取自 spec token 表。
// 自包含 HTML:内联 <style>、零外部资源、仅系统字体栈、无脚本。
// 尺寸统一用 vw/vh 表达(1920×1080 与 32×18 同为 16:9),blur 小视口按同构图等比缩放。
// 换算:1px(1920 宽)= .0521vw;1px(1080 高)= .0926vh;48px 网格 = 2.5vw × 4.4444vh;120px 纹带 = 11.111vh。

const GUN = "#0d1014"; // 枪灰底
const METAL_HI = "#10141a"; // 金属渐变顶
const METAL_LO = "#0a0c10"; // 金属渐变底
const ORANGE = "#f59e0b"; // --accent 警戒橙
const STEEL = "#9aa4b0"; // --mascot-accent 枪灰描边 / 数据读数
const STRIPE_DARK = "#1c2128"; // 警示条纹深灰
const BADGE_FILL = "#14181e"; // mascot 徽章填充

const GRID_INK = "rgba(154,164,176,.08)"; // 工程网格
const NUM_INK = "rgba(154,164,176,.35)"; // "07" 描边
const CROSS_INK = "rgba(255,255,255,.25)"; // 对齐十字
const BRUSH_INK = "rgba(154,164,176,.04)"; // 拉丝细纹
const HEX_INK = "rgba(154,164,176,.05)"; // 六边形网格

const MONO = "Consolas, Menlo, Monaco, 'Courier New', monospace";
// 45° 橙/深灰相间斜纹,24px(=1.25vw)一节,方向全包统一
const HAZARD = `repeating-linear-gradient(45deg,${ORANGE} 0,${ORANGE} 1.25vw,${STRIPE_DARK} 1.25vw,${STRIPE_DARK} 2.5vw)`;

function page(body, bg = GUN) {
  return `<!DOCTYPE html>
<html><head><meta charset="utf-8"><style>
html,body{margin:0;padding:0;width:100%;height:100%;overflow:hidden;background:${bg}}
body{position:relative}
</style></head><body>${body}</body></html>`;
}

// wallpaper-1 四角白色对齐十字:横竖两条 1px 短杠成 +
function alignCross(cx, cy) {
  return `<div style="position:absolute;left:${cx - 0.8}vw;top:calc(${cy}vh - .0463vh);width:1.6vw;height:.0926vh;background:${CROSS_INK}"></div>
<div style="position:absolute;left:calc(${cx}vw - .0261vw);top:${cy - 0.8}vh;width:.0521vw;height:1.6vh;background:${CROSS_INK}"></div>`;
}

// wallpaper-1「蓝图网格」
const blueprint = page(`
<div style="position:absolute;inset:0;background:${GUN}"></div>
<div style="position:absolute;inset:0;background-image:linear-gradient(90deg,${GRID_INK} .0521vw,transparent .0521vw),linear-gradient(0deg,${GRID_INK} .0926vh,transparent .0926vh);background-size:2.5vw 4.4444vh"></div>
<div style="position:absolute;left:12vw;right:12vw;top:24vh;height:.0926vh;background:${ORANGE}"></div>
<div style="position:absolute;left:calc(12vw - .0261vw);top:23.35vh;width:.0521vw;height:1.4vh;background:${ORANGE}"></div>
<div style="position:absolute;right:calc(12vw - .0261vw);top:23.35vh;width:.0521vw;height:1.4vh;background:${ORANGE}"></div>
<div style="position:absolute;top:12vh;bottom:12vh;left:76vw;width:.0521vw;background:${ORANGE}"></div>
<div style="position:absolute;left:75.32vw;top:calc(12vh - .0463vh);width:1.4vw;height:.0926vh;background:${ORANGE}"></div>
<div style="position:absolute;left:75.32vw;bottom:calc(12vh - .0463vh);width:1.4vw;height:.0926vh;background:${ORANGE}"></div>
<div style="position:absolute;inset:0;display:flex;align-items:center;justify-content:center"><span style="font-family:${MONO};font-weight:700;font-size:41.67vw;line-height:1;color:transparent;-webkit-text-stroke:.104vw ${NUM_INK}">07</span></div>
${alignCross(3, 4)}${alignCross(97, 4)}${alignCross(3, 96)}${alignCross(97, 96)}
`);

// wallpaper-2「警示条纹」
const hazardWall = page(`
<div style="position:absolute;inset:0;background:${GUN}"></div>
<div style="position:absolute;left:4vw;right:4vw;top:7vh;height:1.9vh;opacity:.3">
<div style="position:absolute;left:0;right:0;bottom:0;height:.0926vh;background:#ffffff"></div>
<div style="position:absolute;left:0;right:0;bottom:0;height:.8vh;background:repeating-linear-gradient(90deg,#ffffff 0,#ffffff .0521vw,transparent .0521vw,transparent 1.25vw)"></div>
<div style="position:absolute;left:0;right:0;bottom:0;height:1.9vh;background:repeating-linear-gradient(90deg,#ffffff 0,#ffffff .0781vw,transparent .0781vw,transparent 6.25vw)"></div>
</div>
<div style="position:absolute;right:4vw;top:24vh;font-family:${MONO};font-size:1.05vw;line-height:2.2;letter-spacing:.06em;color:${STEEL};text-align:right;white-space:nowrap">SECTOR-07 // PERIMETER SCAN<br>LOAD 062.8% &#183; TEMP 041.2&#176;C<br>GRID 48PX &#183; LOCK ENGAGED</div>
<div style="position:absolute;left:0;right:0;bottom:11.111vh;height:11.111vh;background:${HAZARD}"></div>
`);

// wallpaper-3「金属渐变」:六边形网格与发光线路用内联 SVG(viewBox 等比缩放,blur 同构)
const HEX_PATH = "M48 0 L24 41.57 L-24 41.57 L-48 0 L-24 -41.57 L24 -41.57 Z M120 41.57 L96 83.14 L48 83.14 L24 41.57 L48 0 L96 0 Z";
const TRACE = "M0 650 L420 650 L545 545 L860 545 L985 460";

const metal = page(`
<div style="position:absolute;inset:0;background:linear-gradient(180deg,${METAL_HI} 0%,${METAL_LO} 100%)"></div>
<div style="position:absolute;inset:0;background:repeating-linear-gradient(0deg,${BRUSH_INK} 0,${BRUSH_INK} .0926vh,transparent .0926vh,transparent .2778vh)"></div>
<svg style="position:absolute;inset:0;width:100%;height:100%" viewBox="0 0 1920 1080" preserveAspectRatio="none">
<defs>
<pattern id="hexgrid" width="144" height="83.14" patternUnits="userSpaceOnUse">
<path d="${HEX_PATH}" fill="none" stroke="${HEX_INK}" stroke-width="1.5"/>
</pattern>
<filter id="traceGlow" x="-5%" y="-30%" width="110%" height="160%"><feGaussianBlur stdDeviation="5"/></filter>
</defs>
<rect width="1920" height="1080" fill="url(#hexgrid)"/>
<path d="${TRACE}" fill="none" stroke="${ORANGE}" stroke-width="12" stroke-linejoin="round" opacity=".55" filter="url(#traceGlow)"/>
<path d="${TRACE}" fill="none" stroke="${ORANGE}" stroke-width="3" stroke-linejoin="round"/>
<circle cx="985" cy="460" r="5" fill="${ORANGE}"/>
</svg>
`);

export const wallpapers = [blueprint, hazardWall, metal];

// preview(800×500):HUD 细边框(橙 1px,四角 L 形加粗)+ 中央「警戒工业」等宽小标题 + 警示条纹角标
function hudCorner(pos, borders) {
  return `<div style="position:absolute;${pos};width:3vw;height:6vh;${borders}"></div>`;
}

export const preview = page(`
<div style="position:absolute;inset:0;background:linear-gradient(180deg,${GUN} 0%,${METAL_LO} 135%)"></div>
<div style="position:absolute;inset:0;border:.125vw solid ${ORANGE}"></div>
${hudCorner("left:1.6vw;top:3.2vh", `border-left:.375vw solid ${ORANGE};border-top:.6vh solid ${ORANGE}`)}
${hudCorner("right:1.6vw;top:3.2vh", `border-right:.375vw solid ${ORANGE};border-top:.6vh solid ${ORANGE}`)}
${hudCorner("left:1.6vw;bottom:3.2vh", `border-left:.375vw solid ${ORANGE};border-bottom:.6vh solid ${ORANGE}`)}
${hudCorner("right:1.6vw;bottom:3.2vh", `border-right:.375vw solid ${ORANGE};border-bottom:.6vh solid ${ORANGE}`)}
<div style="position:absolute;left:6vw;top:3.6vh;width:8vw;height:2.2vh;background:${HAZARD}"></div>
<div style="position:absolute;inset:0;display:flex;align-items:center;justify-content:center"><span style="font-family:${MONO},'Microsoft YaHei','PingFang SC',sans-serif;font-weight:700;font-size:3.4vw;letter-spacing:.45em;padding-left:.45em;color:${STEEL};white-space:nowrap">警戒工业</span></div>
`);

// mascot(512×512 透明 PNG):六边形徽章(纵向占高 72%,2px 枪灰描边,填充 #14181e)+ 中心橙色实心 chevron(朝右,占高 45%),扁平
export const mascot = page(`
<svg style="position:absolute;inset:0;width:100%;height:100%" viewBox="0 0 512 512">
<polygon points="256,72 415.4,164 415.4,348 256,440 96.6,348 96.6,164" fill="${BADGE_FILL}" stroke="${STEEL}" stroke-width="2"/>
<polygon points="200,141 310,256 200,371 200,323 262,256 200,189" fill="${ORANGE}"/>
</svg>
`, "transparent");

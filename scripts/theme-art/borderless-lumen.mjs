// borderless-lumen「无界流光」终稿构图(Wave T):teamLab Borderless 式黑底有机光场。
// 唯一设计依据 plans/theme-specs/borderless-lumen.md;色彩全部取自 spec token 表/构图描述。
// 自包含 HTML:内联 <style>、零外部资源、仅系统字体、无脚本。
// 尺寸一律用 vw/vh 表示,保证 1920×1080 与 32×18(blur)两个视口下构图一致。

const VOID = "#040408";
const VIOLET = "#7c5cff"; // --accent
const VIOLET_HI = "#a68bff"; // --accent-hi(淡紫散景/粒子)
const TEAL = "#56e0d4"; // --mascot-accent
const MAGENTA = "#e056a8";
const WHITE = "#ffffff";

function hexA(hex, alpha) {
  const r = parseInt(hex.slice(1, 3), 16);
  const g = parseInt(hex.slice(3, 5), 16);
  const b = parseInt(hex.slice(5, 7), 16);
  return `rgba(${r},${g},${b},${alpha})`;
}

// 确定性伪随机(种子固定,产物可复现)
function rng(seed) {
  let t = seed >>> 0;
  return () => {
    t = (t * 1664525 + 1013904223) >>> 0;
    return t / 4294967296;
  };
}

// 细噪点覆盖层(SVG data-uri,无外部资源):抖动暗部渐变,抑制 JPEG banding
const NOISE =
  "url('data:image/svg+xml,%3Csvg xmlns=%22http://www.w3.org/2000/svg%22 width=%22240%22 height=%22240%22%3E%3Cfilter id=%22n%22%3E%3CfeTurbulence type=%22fractalNoise%22 baseFrequency=%220.9%22 numOctaves=%222%22 stitchTiles=%22stitch%22/%3E%3C/filter%3E%3Crect width=%22240%22 height=%22240%22 filter=%22url(%23n)%22 opacity=%220.55%22/%3E%3C/svg%3E')";

function noiseLayer(opacity) {
  return `<div style="position:absolute;inset:0;background-image:${NOISE};opacity:${opacity}"></div>`;
}

// 柔光色晕:radial-gradient 三站过渡,中段压低避免盘状边界,外缘 70% 处全透明,无硬边
function glow(color, sizeVh, posStyle, opacity) {
  return `<div style="position:absolute;${posStyle};width:${sizeVh}vh;height:${sizeVh}vh;border-radius:50%;background:radial-gradient(circle,${color} 0%,${hexA(color, 0.34)} 40%,transparent 70%);opacity:${opacity}"></div>`;
}

// 散景光点:白色/淡紫小圆,opacity .15-.5,约 1/4 失焦放大(软边缘大圆)
function bokehDots(count, seed) {
  const rand = rng(seed);
  let html = "";
  for (let i = 0; i < count; i += 1) {
    const x = (2 + rand() * 96).toFixed(2);
    const y = (4 + rand() * 90).toFixed(2);
    const opacity = (0.15 + rand() * 0.35).toFixed(2);
    const color = rand() < 0.62 ? WHITE : VIOLET_HI;
    if (rand() < 0.26) {
      const size = (1.6 + rand() * 2.4).toFixed(2); // vw,失焦放大
      html += `<div style="position:absolute;left:${x}vw;top:${y}vh;width:${size}vw;height:${size}vw;border-radius:50%;background:radial-gradient(circle,${hexA(color, 0.9)} 0%,transparent 70%);opacity:${opacity}"></div>`;
    } else {
      const size = (0.22 + rand() * 0.5).toFixed(2); // vw,锐利小点
      html += `<div style="position:absolute;left:${x}vw;top:${y}vh;width:${size}vw;height:${size}vw;border-radius:50%;background:${color};opacity:${opacity}"></div>`;
    }
  }
  return html;
}

// 萤火粒子:1-2px 微点(≈0.05-0.12vw @1920),青/白/紫随机分布,少数带 8px(≈0.42vw)光晕
function fireflies(count, seed) {
  const rand = rng(seed);
  const palette = [TEAL, WHITE, VIOLET_HI];
  let html = "";
  for (let i = 0; i < count; i += 1) {
    const x = (1 + rand() * 98).toFixed(2);
    const y = (1 + rand() * 94).toFixed(2);
    const size = (0.05 + rand() * 0.07).toFixed(3);
    const color = palette[Math.floor(rand() * palette.length)];
    const opacity = (0.45 + rand() * 0.5).toFixed(2);
    const halo = rand() < 0.18 ? `box-shadow:0 0 .42vw ${color};` : "";
    html += `<div style="position:absolute;left:${x}vw;top:${y}vh;width:${size}vw;height:${size}vw;border-radius:50%;background:${color};opacity:${opacity};${halo}"></div>`;
  }
  return html;
}

function page(body, bg = VOID) {
  return `<!DOCTYPE html>
<html><head><meta charset="utf-8"><style>
html,body{margin:0;padding:0;width:100%;height:100%;overflow:hidden;background:${bg}}
body{position:relative}
</style></head><body>${body}</body></html>`;
}

export const wallpapers = [
  // wallpaper-1「花舞光场」:黑底 + 三团错位柔光色晕(紫/青/品红,40-60vh) + 26 枚散景光点
  page(`
<div style="position:absolute;inset:0;background:${VOID}"></div>
${glow(VIOLET, 58, "left:3vw;top:6vh", ".82")}
${glow(TEAL, 47, "right:5vw;top:34vh", ".6")}
${glow(MAGENTA, 42, "left:36vw;top:64vh", ".52")}
${bokehDots(26, 20260717)}
${noiseLayer(".035")}`),

  // wallpaper-2「水镜」:黑底 + 中部 30vh 水平流光带(青→紫→品红 180° 渐变,4.2vh blur 柔化)
  // + 其倒影(scaleY(-1),opacity .3,mask 向下渐隐;倒影先绘、光带压住接缝)
  page(`
<div style="position:absolute;inset:0;background:${VOID}"></div>
<div style="position:absolute;left:-7vw;width:114vw;top:60vh;height:42vh;opacity:.3;-webkit-mask-image:linear-gradient(to bottom,black 0%,black 12%,transparent 82%);mask-image:linear-gradient(to bottom,black 0%,black 12%,transparent 82%)">
<div style="width:100%;height:100%;transform:scaleY(-1);filter:blur(4.2vh);background:linear-gradient(180deg,${TEAL} 0%,${VIOLET} 50%,${MAGENTA} 100%)"></div>
</div>
<div style="position:absolute;left:-7vw;width:114vw;top:34vh;height:30vh;filter:blur(4.2vh);background:linear-gradient(180deg,${TEAL} 0%,${VIOLET} 50%,${MAGENTA} 100%);opacity:.9"></div>
${noiseLayer(".03")}`),

  // wallpaper-3「萤火之森」:黑底 + 偏右 70vh 大紫晕 + 64 枚萤火粒子 + 底部极淡青色地平线辉光
  page(`
<div style="position:absolute;inset:0;background:${VOID}"></div>
${glow(VIOLET, 70, "right:-4vw;top:4vh", ".78")}
${fireflies(64, 8150)}
<div style="position:absolute;left:0;right:0;bottom:0;height:9vh;background:linear-gradient(to top,${hexA(TEAL, 0.16)} 0%,transparent 100%)"></div>
<div style="position:absolute;left:16vw;width:68vw;bottom:1.1vh;height:.16vh;border-radius:1vh;background:${TEAL};opacity:.5;filter:blur(.5vh)"></div>
${noiseLayer(".03")}`),
];

// preview(800×500):单团紫青混合光晕 + 中央「无界流光」白色细体小标题
export const preview = page(`
<div style="position:absolute;inset:0;background:${VOID}"></div>
${glow(VIOLET, 56, "left:24vw;top:10vh", ".85")}
${glow(TEAL, 44, "right:24vw;bottom:6vh", ".62")}
${noiseLayer(".035")}
<div style="position:absolute;inset:0;display:flex;align-items:center;justify-content:center">
<span style="font-family:'Segoe UI','Microsoft YaHei','PingFang SC','Hiragino Sans GB',sans-serif;font-weight:300;font-size:3.4vw;letter-spacing:.3em;color:${WHITE};text-shadow:0 0 1.4vw ${hexA(VIOLET, 0.8)},0 0 .4vw rgba(0,0,0,.65)">无界流光</span>
</div>`);

// mascot(512×512 透明 PNG):三枚 45% 软渐变圆(紫/青/品红)两两交叠,
// screen 混合使重叠区自然混色,各自带 12px 同色 drop-shadow 外缘光晕;有机、无硬边
function orb(color, posStyle) {
  return `<div style="position:absolute;${posStyle};width:45%;height:45%;border-radius:50%;background:radial-gradient(circle,${color} 0%,${hexA(color, 0.55)} 45%,transparent 72%);mix-blend-mode:screen;filter:drop-shadow(0 0 12px ${hexA(color, 0.85)})"></div>`;
}

export const mascot = page(`
${orb(VIOLET, "left:11%;top:9%")}
${orb(TEAL, "right:11%;top:15%")}
${orb(MAGENTA, "left:27.5%;bottom:10%")}`, "transparent");

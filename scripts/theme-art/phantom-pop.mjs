// phantom-pop「魅影波普」终稿构图(Wave T):激进斜切 + 黑/红/白撞色 + halftone 网点 + 撕裂拼贴。
// 设计语言与验收见 plans/theme-specs/phantom-pop.md(只取 P5 波普语言,无商标造型)。
// 自包含 HTML:内联 <style>、零外部资源、仅系统字体栈、无脚本。
// 尺寸说明:管线主渲染为 1920×1080,spec 的 px 参数统一换算为 vh
// (12px = 1.111vh,8px = .741vh,24px = 2.222vh),使 32×18 的 blur 小视口重渲染保持同构图。

const BLACK = "#0a0809"; // 主黑底
const BLACK_DEEP = "#3d0b10"; // 深红渐变端
const RED = "#e6242f"; // --accent
const WHITE = "#ffffff"; // --mascot-accent
const INK_GRAY = "#1a1617"; // 墨灰拼贴块

// 通用八角爆炸形(自绘 clip-path polygon,外/内半径交替的 8 尖角,非商标造型)。
const EXPLOSION =
  "polygon(50% 0%,63.8% 16.7%,85.4% 14.6%,83.3% 36.2%,100% 50%,83.3% 63.8%,85.4% 85.4%,63.8% 83.3%,50% 100%,36.2% 83.3%,14.6% 85.4%,16.7% 63.8%,0% 50%,16.7% 36.2%,14.6% 14.6%,36.2% 16.7%)";

// 白色实心五角星(自绘 polygon)。
const STAR = "polygon(50% 0%,61% 35%,98% 35%,68% 57%,79% 91%,50% 70%,21% 91%,32% 57%,2% 35%,39% 35%)";

const MONO = "'Courier New',ui-monospace,SFMono-Regular,Menlo,Consolas,monospace";
const SANS = "'Microsoft YaHei','PingFang SC','Noto Sans CJK SC','Segoe UI',sans-serif";

// halftone 网点:rgba(255,255,255,.06) 圆点,24px(2.222vh)网格。
const HALFTONE =
  "background-image:radial-gradient(circle,rgba(255,255,255,.06) .19vh,transparent .21vh);background-size:2.222vh 2.222vh";

// feTurbulence 噪点 data-uri(wallpaper-3 全幅纹理,叠加层 opacity .05)。
const NOISE_SVG = `<svg xmlns='http://www.w3.org/2000/svg' width='300' height='300'><filter id='n'><feTurbulence type='fractalNoise' baseFrequency='0.8' numOctaves='3' stitchTiles='stitch'/><feColorMatrix type='matrix' values='0 0 0 0 1 0 0 0 0 1 0 0 0 0 1 0 0 0 1 0'/></filter><rect width='300' height='300' filter='url(#n)'/></svg>`;
const NOISE_URI = `data:image/svg+xml,${NOISE_SVG.replace(/#/g, "%23")
  .replace(/</g, "%3C")
  .replace(/>/g, "%3E")
  .replace(/ /g, "%20")}`;

function page(css, body, bg = BLACK) {
  return `<!DOCTYPE html>
<html><head><meta charset="utf-8"><style>
html,body{margin:0;padding:0;width:100%;height:100%;overflow:hidden;background:${bg}}
body{position:relative}
${css}
</style></head><body>${body}</body></html>`;
}

// 斜切色带(-18°):40vh 红色主带 + 边缘 12px(1.111vh)白色平行细带。
function slashBand(top, redHeight, stripHeight) {
  return `<div style="position:absolute;left:-30vw;top:${top};width:160vw;transform:rotate(-18deg)">
<div style="height:${redHeight};background:${RED}"></div>
<div style="height:${stripHeight};background:${WHITE}"></div>
</div>`;
}

// wallpaper-1「斜切红白黑」:黑底 + halftone 网点 + -18° 红带/白细带 + 角落 -8° 白色 mono 两行小字。
const wallpaper1 = page(
  "",
  `
<div style="position:absolute;inset:0;${HALFTONE}"></div>
${slashBand("46vh", "40vh", "1.111vh")}
<div style="position:absolute;right:5vw;bottom:7vh;transform:rotate(-8deg);text-align:right;font-family:${MONO};font-size:1.7vh;letter-spacing:.2em;line-height:2;color:${WHITE};opacity:.7">PHANTOM POP<br>BLACK / RED / WHITE</div>`,
);

// wallpaper-2「拼贴框」:黑底 + 红实心 / 白描边 / 墨灰实心(带 halftone 阴影)三旋转矩形错位叠放,
// 中央偏右白色八角爆炸形。
const wallpaper2 = page(
  "",
  `
<div style="position:absolute;left:17.4vw;top:24.8vh;width:30vw;height:50vh;transform:rotate(-2deg);background-image:radial-gradient(circle,rgba(255,255,255,.20) .22vh,transparent .25vh);background-size:2.2vh 2.2vh"></div>
<div style="position:absolute;left:12vw;top:15vh;width:28vw;height:55vh;transform:rotate(-8deg);background:${RED}"></div>
<div style="position:absolute;left:16vw;top:22vh;width:30vw;height:50vh;transform:rotate(-2deg);background:${INK_GRAY}"></div>
<div style="position:absolute;left:21vw;top:13vh;width:27vw;height:52vh;transform:rotate(5deg);box-sizing:border-box;border:1.1vh solid ${WHITE}"></div>
<div style="position:absolute;right:13vw;top:50%;width:26vw;height:26vw;transform:translateY(-50%) rotate(8deg);background:${WHITE};clip-path:${EXPLOSION}"></div>`,
);

// wallpaper-3「红黑渐变噪点」:#0a0809 → #3d0b10 对角渐变 + 全幅 feTurbulence 噪点(.05)
// + 8px(.741vh)白色 24° 对角细线贯穿。
const wallpaper3 = page(
  `.noise{position:absolute;inset:0;background-image:url("${NOISE_URI}");background-size:300px 300px;opacity:.05}`,
  `
<div style="position:absolute;inset:0;background:linear-gradient(135deg,${BLACK} 0%,${BLACK_DEEP} 100%)"></div>
<div class="noise"></div>
<div style="position:absolute;left:-25vw;top:38vh;width:150vw;height:.741vh;background:${WHITE};transform:rotate(24deg)"></div>`,
);

export const wallpapers = [wallpaper1, wallpaper2, wallpaper3];

// preview(800×500):wallpaper-1 变体(缩小斜切带 + halftone + 左上白色小爆炸形)
// + 底部黑色信息条(红色斜切角块 + 白色粗体小标题「魅影波普」)。
export const preview = page(
  "",
  `
<div style="position:absolute;inset:0;${HALFTONE}"></div>
${slashBand("34vh", "34vh", "1.2vh")}
<div style="position:absolute;left:8vw;top:10vh;width:16vw;height:16vw;transform:rotate(12deg);background:${WHITE};clip-path:${EXPLOSION}"></div>
<div style="position:absolute;left:0;right:0;bottom:0;height:16%;background:${BLACK};display:flex;align-items:center">
<div style="height:100%;width:13%;background:${RED};clip-path:polygon(0 0,100% 0,70% 100%,0 100%)"></div>
<span style="margin-left:5%;font-family:${SANS};font-weight:800;font-size:4.8vh;letter-spacing:.08em;color:${WHITE}">魅影波普</span>
<span style="margin-left:4%;font-family:${MONO};font-size:1.9vh;letter-spacing:.22em;color:${RED}">PHANTOM POP</span>
</div>`,
);

// mascot(512×512 透明 PNG):黑色八角爆炸形(边长 70%)+ 10px 红色粗边(外层红形 scale .944 内层黑形)
// + 中心白色实心五角星。扁平高对比。
export const mascot = page(
  "",
  `
<div style="position:absolute;left:15%;top:15%;width:70%;height:70%;background:${RED};clip-path:${EXPLOSION}">
<div style="position:absolute;inset:0;background:${BLACK};clip-path:${EXPLOSION};transform:scale(.944)"></div>
<div style="position:absolute;left:29%;top:29%;width:42%;height:42%;background:${WHITE};clip-path:${STAR}"></div>
</div>`,
  "transparent",
);

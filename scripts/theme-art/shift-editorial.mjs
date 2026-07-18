// shift-editorial「素纸编集」终稿构图(Wave T):纸面编辑美学 —— 米白纸底、发丝线、超大墨色衬线标题、单一信号红点睛、克制的不对称网格。
// 唯一设计依据:plans/theme-specs/shift-editorial.md(色值严格取自其 token 表)。
// 自包含 HTML:内联样式、零外部资源、仅系统字体栈、无脚本。

const PAPER = "#f7f4ee"; // --bg-deep,壁纸纸底
const PAPER_DEEP = "#e4ded0"; // wallpaper-3 渐变终点(spec 构图描述)
const ELEV = "#fffefa"; // --bg-elev,preview 白色信息条
const INK = "#18150f"; // --text-primary,墨色
const ACCENT = "#d4293c"; // --accent,信号红
const MUTED = "#8a8272"; // --text-muted,等宽小字灰
const HAIRLINE = "rgba(24,21,15,.18)"; // spec 发丝线(wallpaper-2 密排线)
const BORDER = "rgba(24,21,15,.14)"; // --border

// 注意:字体栈内含空格的字体名用单引号 —— 双引号会截断 style 属性
const SERIF = `Georgia,'Times New Roman',serif`;
const MONO = `Consolas,'Courier New',monospace`;

function page(body, bg = PAPER) {
  return `<!DOCTYPE html>
<html><head><meta charset="utf-8"><style>
html,body{margin:0;padding:0;width:100%;height:100%;overflow:hidden;background:${bg}}
body{position:relative}
</style></head><body>${body}</body></html>`;
}

// 红色 8×8px 对齐标记(registration mark)
function regMark(pos) {
  return `<div style="position:absolute;${pos};width:8px;height:8px;background:${ACCENT}"></div>`;
}

// wallpaper-1「横排大标题」共享构图(preview 复用其缩小变体)
function editorialTitle({ marks = true, monoTop = true, monoBottom = true, titleTop = "50%" } = {}) {
  return `
<!-- 左侧竖排 1px 发丝栏线三条(递减长度,不对称网格) -->
<div style="position:absolute;left:5vw;top:12vh;width:1px;height:76vh;background:${HAIRLINE}"></div>
<div style="position:absolute;left:7vw;top:12vh;width:1px;height:57vh;background:${HAIRLINE}"></div>
<div style="position:absolute;left:9vw;top:12vh;width:1px;height:38vh;background:${HAIRLINE}"></div>
<!-- 右侧出血的超大墨色衬线字母 MOE,字高约 60vh -->
<div style="position:absolute;right:-6vw;top:${titleTop};transform:translateY(-50%);font-family:${SERIF};font-weight:bold;font-size:60vh;line-height:1;letter-spacing:-.02em;color:${INK}">MOE</div>
${marks ? `${regMark("left:24px;top:24px")}${regMark("right:24px;top:24px")}${regMark("left:24px;bottom:24px")}${regMark("right:24px;bottom:24px")}` : ""}
${monoTop ? `<div style="position:absolute;left:40px;top:23px;font-family:${MONO};font-size:10px;letter-spacing:.14em;color:${MUTED}">MOE-PLAY · VOL.01</div>` : ""}
${monoBottom ? `<div style="position:absolute;right:40px;bottom:23px;font-family:${MONO};font-size:10px;letter-spacing:.14em;color:${MUTED}">SHIFT EDITORIAL — FIG.01</div>` : ""}`;
}

export const wallpapers = [
  // wallpaper-1「横排大标题」
  page(editorialTitle()),

  // wallpaper-2「网格与留白」:左 2/3 留白,右 1/3 密排水平发丝线(间距 18px),唯一色点 + 底部贯穿发丝线
  page(`
<div style="position:absolute;left:66.67%;top:0;bottom:0;width:33.33%;background:repeating-linear-gradient(0deg,${HAIRLINE} 0,${HAIRLINE} 1px,transparent 1px,transparent 18px)"></div>
<div style="position:absolute;left:55%;top:36%;width:48px;height:48px;background:${ACCENT}"></div>
<div style="position:absolute;left:0;right:0;bottom:10%;height:1px;background:${HAIRLINE}"></div>`),

  // wallpaper-3「墨色渐变」:纸底到 #e4ded0 垂直淡渐变 + 6px 红色 35° 对角粗线(左下→右上) + 角落浅灰 halftone 点阵
  page(`
<div style="position:absolute;inset:0;background:linear-gradient(180deg,${PAPER} 0%,${PAPER_DEEP} 100%)"></div>
<div style="position:absolute;left:0;top:0;width:24vw;height:32vh;background-image:radial-gradient(circle,rgba(24,21,15,.10) 1.6px,transparent 1.7px);background-size:16px 16px"></div>
<div style="position:absolute;left:10vw;top:95vh;width:83vw;height:6px;background:${ACCENT};transform-origin:left center;transform:rotate(-35deg)"></div>`),
];

// preview(800×500):wallpaper-1 缩小变体 + 底部白色信息条(发丝线上边框 + 黑色小标题「素纸编集」+ 红色小方块)
export const preview = page(`
${editorialTitle({ marks: false, monoBottom: false, titleTop: "44%" })}
${regMark("left:24px;top:24px")}${regMark("right:24px;top:24px")}
<div style="position:absolute;left:0;right:0;bottom:0;height:14%;background:${ELEV};border-top:1px solid ${BORDER};display:flex;align-items:center;gap:14px;padding:0 24px;box-sizing:border-box">
<span style="flex:none;width:14px;height:14px;background:${ACCENT}"></span>
<span style="font-family:${SERIF};font-weight:bold;font-size:20px;letter-spacing:.06em;color:${INK}">素纸编集</span>
<span style="margin-left:auto;font-family:${MONO};font-size:10px;letter-spacing:.14em;color:${MUTED}">MOE-PLAY THEME PACK</span>
</div>`);

// mascot(512×512 透明 PNG):红色实心圆(直径 55%)居中,墨色 2px 发丝十字超出圆缘,圆外 3px 墨色细方框(边长 88%),扁平无渐变
export const mascot = page(`
<div style="position:absolute;left:50%;top:50%;width:55%;height:55%;transform:translate(-50%,-50%);border-radius:50%;background:${ACCENT}"></div>
<div style="position:absolute;left:50%;top:50%;width:72%;height:2px;transform:translate(-50%,-50%);background:${INK}"></div>
<div style="position:absolute;left:50%;top:50%;width:2px;height:72%;transform:translate(-50%,-50%);background:${INK}"></div>
<div style="position:absolute;left:6%;top:6%;width:88%;height:88%;box-sizing:border-box;border:3px solid ${INK}"></div>`, "transparent");

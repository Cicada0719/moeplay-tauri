# MoePlay SHIFTBRAIN Concept

独立网页版概念站，用于评审下一版 MoePlay 的产品骨架和三种导演风格。它不加载 Tauri，不修改生产入口，也不共享生产状态。

## 运行

```bash
npm install
npm run dev:concept
```

打开 `http://127.0.0.1:1420/concept/index.html`。

构建与测试：

```bash
npm run check
npm run build:concept
npm run test:concept
```

## 结构

- **SHIFTBRAIN 骨架**：固定模块导航、模板切换、Visual / Index / Scene 控制器、媒体指针、输入意图映射。
- **01 Cinematic**：电影档案、时间码、黑场与帧序列。
- **02 Editorial**：暗色 ACG 画报、文字优先索引、跨页拼贴。
- **03 Kinetic**：连续媒体流、拖拽/滚轮/触摸目标位置、Three.js 媒体舞台。
- **Detail**：三模板拥有独立的详情构图，而非共享换皮弹层。
- **Review Panel**：模板、模块、模式、动态质量、声音和预览 viewport 实时切换。

## 输入

- 键盘：方向键移动，Enter/Space 打开，Esc 返回，Shift/Alt + 左右切模式，1/2/3 直达模式，R 开关评审面板。
- 鼠标/触控板：Visual 仅在媒体意图区接管；Index 保留原生滚动；Scene 具有阈值和冷却。
- 触摸：单指滑动转换为前后意图。
- 手柄：D-pad/摇杆移动，A 确认，B 返回，LB/RB 切换模式。

## 资源规则

演示媒体全部位于 `public/concept/assets/`，运行时不热链。来源、构图、尺寸、焦点、用途、字节数和 SHA-256 记录在 `public/concept/media-manifest.json`。界面美术不使用 SVG，也没有 AI 生成素材。

## 边界

本目录是独立概念验证；确认前不向 `src/`、`src-tauri/`、漫画源或版本号迁移。


# 萌游 MoeGame

跨平台游戏与番剧管理器，基于 Tauri 2 + Svelte 5 构建。

统一管理你的 Steam / Epic 游戏库、追番记录、弹幕播放、漫画阅读，一个应用搞定，借鉴接入kazumi 源规则，哔咔源。

## 功能概览

**游戏管理**
- Steam / Epic / 模拟器游戏自动导入
- 多源元数据刮削（Bangumi · VNDB · DLSite · Steam · PCGW 等）
- 智能合集、标签、评分、笔记
- 全局手柄优先导航（普通页面 + PS5 风格大屏模式）
- 游戏时间统计与成就追踪

**番剧播放**
- 多规则源搜索，并行换源
- 视频流嗅探 + 本地代理播放
- HLS.js / 原生双模自动兜底
- 弹幕叠加（弹幕库对接）
- 本地 GPU 超清化（均衡 1.5× / 质量优先 2×，最高 1080p/1440p）
- 画中画、倍速、手势控制、外部播放器调用
- trace.moe 截图识番

**漫画阅读**
- 在线漫画源聚合
- 收藏与阅读进度同步

## 技术栈

| 层 | 技术 |
|---|---|
| 桌面框架 | Tauri 2 (Rust + WebView2) |
| 前端 | Svelte 5 (Runes) + TypeScript + Vite |
| 后端 | Rust (reqwest · ureq · rusqlite · scraper) |
| 视频 | HLS.js · 本地 HTTP 代理 · WebView 嗅探 |
| 存储 | SQLite（本地）|
| 样式 | Scoped CSS · CSS Variables |

## 环境要求

- **Node.js** >= 18
- **Rust** >= 1.75（stable）
- **Windows 10/11**（当前仅支持 Windows，需要 WebView2 Runtime）

## 开发

```bash
# 安装依赖
npm install

# 启动开发服务器（Vite + Tauri）
npm run tauri dev

# 类型检查
npm run check

# 单元测试
npm run test:unit
```

## 构建

```bash
# 生产构建（MSI + NSIS 安装包）
npm run tauri build

# 便携版打包
npm run package:portable
```

构建产物在 `src-tauri/target/release/bundle/` 目录下。

## 项目结构

```
moeplay-tauri/
├── src/                    # 前端源码
│   ├── lib/
│   │   ├── components/     # Svelte 组件
│   │   │   ├── anime/      # 番剧相关（播放器、详情、选源、弹幕）
│   │   │   ├── comic/      # 漫画相关
│   │   │   ├── switch/     # 大屏模式
│   │   │   └── ui/         # 通用 UI 组件
│   │   ├── stores/         # Svelte 5 状态管理
│   │   └── utils/          # 工具函数
│   └── app.css             # 全局样式
├── src-tauri/              # Rust 后端
│   ├── src/
│   │   ├── commands/       # Tauri IPC 命令
│   │   ├── scraper/        # 元数据刮削器
│   │   ├── anime.rs        # 番剧规则引擎
│   │   ├── video_extractor.rs  # 视频流嗅探
│   │   ├── video_proxy.rs  # 本地视频代理
│   │   └── ...
│   └── tauri.conf.json     # Tauri 配置
├── .github/workflows/      # CI/CD
└── scripts/                # 构建脚本
```

## 许可证

[MIT](LICENSE)

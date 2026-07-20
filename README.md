# 萌游 MoeGame

<div align="center">

**面向 Windows、电视大屏与手柄操作的本地游戏 / 番剧 / 漫画中心**

[下载最新版](https://github.com/Cicada0719/moeplay-tauri/releases/latest) · [查看更新记录](CHANGELOG.md) · [提交问题](https://github.com/Cicada0719/moeplay-tauri/issues)

</div>

![萌游 MoeGame 大屏滚动剧场](docs/screenshots/big-picture-release.png)

## 这是什么

萌游把本地游戏库、游玩记录、番剧播放和漫画阅读放进同一个桌面应用。它不是传统的文件列表，而是一套同时适配鼠标、键盘和手柄的媒体界面：桌面上可以高效管理内容，连接电视后可直接进入全屏大屏模式。

当前正式发布平台为 **Windows 10/11 x64**。项目暂不发布 Android 版本，也不会在 Release 中提供 APK 或 AAB。

## 主要能力

### 游戏库

- 导入 Steam、Epic、本地游戏与模拟器内容
- 使用 Bangumi、VNDB、DLSite、Steam、PCGamingWiki 等来源补全资料
- 管理封面、背景、标签、合集、评分、笔记和启动参数
- 记录游玩时长、最近启动与继续游玩内容
- 支持搜索、筛选、批量操作和游戏档案

### 大屏模式与手柄

- 与普通游戏库完全不同的电视端全屏作品舞台
- 背景图扩散覆盖整个屏幕，标题与说明收纳在左下角
- 底部封面滚轮由左摇杆或 D-Pad 手动切换，不自动轮播
- 游戏库、全部内容、搜索、媒体展厅和档案之间支持手柄焦点往返
- 检测到手柄后显示对应按键提示，并为内部操作按钮提供焦点状态
- 支持折叠次要控件，让单一功能场景获得更大的内容显示空间

### 番剧播放

- 规则源并行搜索、选源、换源与失效回退
- HLS 与原生播放双通道，本地代理处理兼容性问题
- 弹幕、倍速、画中画、外部播放器与截图识番
- 本地 GPU 超清化：均衡 1.5×、质量优先 2×，可按设备性能提高输出分辨率
- 播放进度与收藏状态保存在本地

### 漫画与统一内容页

- 聚合漫画源、收藏和阅读进度
- 游戏、番剧、漫画页面共享统一的手柄导航与响应式规则
- 在小窗口、1080p、4K 和超宽屏下自动调整信息密度
- 支持系统减少动态效果设置

## 下载与自动更新

前往 [GitHub Releases](https://github.com/Cicada0719/moeplay-tauri/releases/latest) 下载：

- **NSIS 安装版**：推荐大多数用户使用，可接收应用内自动更新
- **MSI 安装版**：适合 Windows Installer 部署场景
- **Portable 便携版**：解压后运行，不写入系统安装记录

正式 Release 只有在安装包、更新包、分离签名和 `latest.json` 全部通过校验后才会公开。客户端固定读取 GitHub 最新正式版更新清单，避免打包版无法发现新版本或误用未签名安装包。

> Windows 需要 WebView2 Runtime。Windows 10/11 通常已随系统或 Edge 安装。

## 数据与隐私

- 游戏资料、收藏、进度、记录和设置默认保存在本机 SQLite 数据库
- 只有搜索资料、获取媒体资源、同步平台库或检查更新时才访问对应网络服务
- 密钥、令牌和本地数据库不会提交到仓库
- 下载、代理和日志输出会经过路径与敏感字段保护

## 开发环境

- Node.js 20
- Rust stable
- Windows 10/11 x64
- Microsoft WebView2 Runtime
- Visual Studio C++ Build Tools（Tauri / Rust Windows 构建）

```powershell
# 安装依赖
npm ci

# 启动桌面开发环境
npm run tauri dev

# 静态检查
npm run check

# 单元测试
npm run test:unit

# 前端正式构建
npm run build

# Windows Tauri 打包
npm run tauri build
```

## 发布质量门槛

每次正式发布会在 GitHub Actions 中执行：

1. JavaScript / Rust 依赖与许可证审计
2. 版本号、Tauri 命令契约和更新策略校验
3. Rust 格式、Clippy 与完整测试
4. Svelte / TypeScript 检查和前端单元测试
5. 正式构建、体积预算和 Playwright 界面测试
6. Windows 安装包、Portable、SBOM、构建元数据和发布清单生成
7. 自动更新签名与 `latest.json` 一致性验证

任一步骤失败，Release 会保持草稿或直接失败，不会成为客户端可见的最新版。

## 项目结构

```text
src/                         Svelte 5 前端、页面、状态与手柄导航
src-tauri/                   Rust 后端、数据库、抓取、代理和 Tauri 配置
plugins/                     项目内 Tauri 插件
scripts/                     构建、审计、更新和发布校验脚本
tests/                       Playwright 界面与响应式测试
docs/screenshots/            Release 与 README 使用的正式界面截图
.github/workflows/            Windows CI、夜间任务和签名发布流程
```

## 许可证

本项目使用 [MIT License](LICENSE)。第三方内容源、游戏封面、番剧与漫画内容的权利归各自权利人所有。

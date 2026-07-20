# 萌游 MoeGame v0.19.1

![萌游 MoeGame 大屏滚动剧场](https://github.com/Cicada0719/moeplay-tauri/releases/download/{{TAG}}/big-picture-release.png)

## 大屏滚动剧场

- 大屏模式与普通游戏库彻底差异化，采用面向电视和手柄的全屏作品舞台。
- 标题、说明与启动操作缩小并移动到左下角，减少对背景画面的遮挡。
- 主要游戏封面改为底部横向滚轮，使用左摇杆或 D-Pad 左右手动切换，不自动轮播。
- 背景作品图扩散覆盖整个屏幕，并针对仅有竖版封面的游戏提供全屏模糊综合色场。
- 补齐顶部导航、封面轮、信息操作区、搜索、媒体展厅和游戏档案之间的手柄焦点往返。

## 仓库与发布整理

- 全新重写 README，直接展示本次大屏界面和 Windows 安装 / 自动更新说明。
- 清除已被正式界面取代的 Concept 原型、演示素材、旧版本规划包、交接报告和历史构建脚本。
- 删除停用的 Android CI / Release job 以及签名辅助残留；本版本不会构建或上传 APK / AAB。
- Release 页面会同时上传 1920×1080 界面截图、SBOM、构建元数据和签名更新资源。

## 验证

- Svelte / TypeScript：0 errors / 0 warnings
- Frontend unit：653 passed / 1 skipped
- 大屏纯手柄流程：5 passed
- 1080p、4K、1280×720、21:9 与 reduced-motion 布局：7 passed
- Rust：完整测试通过
- Windows NSIS、MSI、Portable、SBOM、构建元数据与发布清单：通过

## 下载与更新

本版本仅发布 Windows x64 的 NSIS、MSI、便携包和签名自动更新资源。安装版客户端可通过 `latest.json` 自动检查并安装更新。

**不包含 APK 或 AAB。**

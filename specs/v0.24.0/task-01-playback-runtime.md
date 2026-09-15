# Task 01：解析生命周期与真实取消

## 目标与依赖

在现有播放解析基础上让取消真正贯通 Rust 任务、网络请求和隐藏解析窗口。无前置任务；只服务 Task 02，不改播放器 UI。

## 允许修改

`src-tauri/src/video_extractor.rs`、番剧解析/搜索相关 command、`src-tauri/src/lib.rs` 命令注册与必要权限、对应 `src/lib/api/` 封装、Rust/前端单测。禁止修改播放器状态机、规则文件、阅读历史和下载 UI。

## 接口与行为

- `anime_extract_video_url` 增加可选 `session_id`/`scope`，旧调用省略时保持兼容。
- 增加 `anime_cancel_extract(scope)`；调用后解析返回 `cancelled`，不得回退传统解析或发布结果。
- 复用已有 `CancellationToken` scope；同一 scope 新任务自动取消旧任务。
- 隐藏 WebView 最多两个槽位：前台一个、预取一个；空闲槽静音、停止加载并导航空白页后释放引用，窗口数量不得无界增长。
- 解析结果必须携带 `session_id`、最终页面 URL（若有）和来源信息。

## 实现步骤

1. 清点现有提取调用和 scope，先补类型与命令契约测试。
2. 将 token 传入嗅探、网络等待和传统回退边界；每个 await 后检查取消。
3. 将 WebView 创建/复用/清理集中在有界池，关闭或取消时执行清理。
4. 记录前台与预取的生命周期指标，不记录 URL 为成功。

## 验收与完成条件

- 连续切换/取消 50 次，隐藏窗口数量始终不超过 2。
- 只有最后会话能发布 URL；取消后不得出现旧声音或旧回调覆盖。
- 旧无 `session_id` 调用仍通过契约测试。
- Rust 单测、命令契约、fmt、Clippy、前端 check 通过；PR 描述列出未覆盖的平台差异。


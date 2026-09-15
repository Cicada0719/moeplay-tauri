# Task 02：播放状态、换源与错误终态

## 目标与依赖

依赖 Task 01。修复“有声音无画面、一直尝试备用源、快速换源串播、集数错配”，复用 v0.23.1 无帧监控，不重做视频帧检测。

## 允许修改

`src/lib/stores/anime.svelte.ts`、`src/lib/components/anime/AnimePlayer.svelte`、来源选择/剧集匹配组件、`src/lib/utils/animeSource.ts` 及相关测试。Task 01 的 Rust 契约只能按兼容方式调用；禁止修改规则包和阅读历史。

## 接口与状态

`PlaybackSession = { sessionId, attempt, contentId, episodeId, sourceId, resumeSeconds }`；内部事件为 `first-frame`、`error`、`source-exhausted`、`cancelled`。错误包含 `stage: extract|http|media|frame`、`kind`、可选 `httpStatus`、`retryable`。

## 实现步骤

1. 建立单一播放会话状态，所有定时器、retry、预取和换源响应检查会话身份。
2. 候选请求并发最多 3 个；每次解析上限 20 秒；最多自动尝试 3 个不同来源；成功候选立即尝试。
3. HLS 保留 `response.code/status`；HTTP 403、解析空、超时、解码失败和无帧分开显示。
4. 源切换用现有匹配函数按作品/季/明确剧集号匹配；不确定时打开选择面板，不跳到其他集。
5. 候选耗尽进入 terminal 状态，停止加载并提供重试、手动换源、复制诊断；保留可恢复进度。

## 验收

- 第 5 集 12:30 换源后恢复误差 ≤10 秒；快速连续切换只最后会话生效。
- 纯音频、黑屏和无帧不记录成功；特别篇不会冒充正片。
- 3 个候选均失败后不再无限转圈；关闭播放器无残留声音。
- Playwright/单元测试覆盖错误映射、竞态、恢复位置和候选上限；完成仓库质量门槛。


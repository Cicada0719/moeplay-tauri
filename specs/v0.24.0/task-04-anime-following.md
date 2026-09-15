# Task 04：追番更新中心

## 目标与依赖

依赖 Task 02、03。在现有收藏、Bangumi 日历和历史卡片之上增加实际来源追更、未看统计和一键续播。

## 允许修改

`src/lib/features/anime-home/collection.svelte.ts`、`historyStore.svelte.ts`、Anime 首页/掌机入口、详情/来源选择组件和新增追更模块。播放器只接入已定义事件；禁止修改 AnimePlayer 核心和规则引擎。

## 数据与规则

持久化 `FollowingItem { key, contentId, sourceId, seasonKey, knownEpisodes[], watchedEpisodeIds[], baselineReady, pendingNoticeIds[], status, lastCheckedAt, errorKind }`。key 必须含来源、作品和季；标题不作为唯一键。

- 以实际播放来源的剧集列表检查；Bangumi 只作日历和元数据辅助。
- 首次检查建立基线，新增提醒为 0；未看数仍为已知剧集减已看集合。
- 默认手动检查；开启自动后仅应用运行时每 30 分钟检查，退后台暂停，关闭应用不保证通知。
- 播放结束或有效进度 ≥90% 默认标记已看，手动标记可覆盖；来源失败保留旧数据并显示 `unknown`。

## 验收

新增一集只提醒一次；特别篇、改名和同名不同季隔离；检查失败不清空旧未看数；一键续播优先最近未完成记录，其次首个未看集，无源时打开选源面板。覆盖 store 单测和桌面/掌机 UI 测试。


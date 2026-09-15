# Task 05：离线内容存储与下载关联

## 目标与依赖

无新增前置，依赖现有 Task Queue/Downloader。建立可恢复的漫画/小说章节离线库，不创建第二套下载器。

## 允许修改

新增 `src-tauri/src/offline/`、`src-tauri/src/commands/offline.rs`、必要的 `lib.rs` 命令/权限和下载任务 metadata 字段。禁止修改源解析算法、番剧下载协议和阅读位置存储。

## 接口与存储

目录为 `<app_data>/moeplay/offline/`；章节写入临时目录，完成后 rename。清单至少含 `contentType/sourceId/contentId/chapterId/order/title/state/bytes/resources`。

提供：`offline_enqueue`、`offline_supply_chapter`、`offline_list`、`offline_get_chapter`、`offline_control`、`offline_stats`。`offline_control` 支持 pause/resume/retry/cancel/delete；`offline_supply_chapter` 接收前端解析得到的正文或图片资源描述。

- 每个任务 metadata 含 `offlineBundleId/offlineChapterKey/resourceKey/savePath`。
- 启动 hydrate 未完成任务；缺文件或损坏清单回到可恢复状态。
- 图片全部齐备、小说正文完整后才是 `complete`；重复章节幂等。
- 目录路径由内部 ID 生成，删除操作限制在离线根目录；复用磁盘预检和断点续传。

## 验收

中断、断网、重启、缺文件、磁盘不足和重复添加均可恢复或明确失败；删除章节/作品后统计准确；下载任务不会产生阅读历史。新增 Rust 命令同步更新契约与权限测试。


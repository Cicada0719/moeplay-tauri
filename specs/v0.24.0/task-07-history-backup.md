# Task 07：番剧、漫画、小说历史备份

## 目标与依赖

依赖 Task 04、06。统一导出三类本地历史，不承担 WebDAV 同步，不接入尚未注册的 SQLite 写入命令。

## 允许修改

新增 `src/lib/features/media-history/backup.ts`；改 UnifiedMediaHistory、reading repository 和番剧 history store 的导出/导入适配。禁止修改 SQLite 迁移、WebDAV 合并和下载内容。

## 格式与接口

```json
{ "format":"moeplay-media-history", "version":1, "exportedAt":0,
  "coverage":["anime","manga","novel"], "anime":[], "manga":[], "novel":[] }
```

番剧保留 source、作品、集、线路和毫秒进度；漫画保留 chapter/pageIndex/pageId；小说保留 chapter/progress。导出包含有效记录和按类型错误统计，不含凭据、令牌或离线内容。

- 导入接受该 envelope、现有 `moeplay-reading-history` v2 和可明确识别的旧 JSON 数组；无法识别的记录跳过并计数。
- 预览新增/更新/跳过；按稳定身份和时间合并，同时间保留当前记录，不删除现有数据。
- 分别写入 IndexedDB 和番剧 localStorage；成功写入后才计入 imported。
- 存储故障时尽可能导出内存、pending 和仍可读取的旧记录，并报告缺失范围。

## 验收

三类导出到干净环境后位置一致；重复导入幂等；坏记录不阻塞有效记录；三类身份不串；导出文本中不存在密码或下载文件内容。


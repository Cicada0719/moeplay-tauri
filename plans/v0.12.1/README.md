# MoePlay 0.12.1 Development Plan Index

本目录是以 2026-07-10 当前代码为证据生成的 0.12.1 规划包。

## 基线警告

仓库当前实际版本仍为 **0.12.0**，没有 `v0.12.1` tag；当前 `HEAD=3ceb354` 包含 3 个尚未打入正式版本的媒体可靠性提交。因此第一步不是直接堆功能，而是把当前 HEAD 固化为可升级、可回滚、权限一致的 0.12.1 RC 基线。

## 阅读顺序

1. [MASTER-PLAN.md](./MASTER-PLAN.md)  
   产品目标、架构、阶段、量化预算、发布门禁与完成定义。
2. [AGENT-WORK-PACKAGES.md](./AGENT-WORK-PACKAGES.md)  
   10 个代理工作包、独占写入范围、依赖、合并顺序和回报模板。
3. [spec-05-platform-quality.md](./spec-05-platform-quality.md)  
   先做的平台 P0：命令权限漂移、数据库恢复安全、Secret、任务、CI/发布。
4. [spec-01-media-sources.md](./spec-01-media-sources.md)  
   番剧/漫画统一源契约、内部播放阅读、源优先级、熔断与回退。
5. [spec-02-library-dashboard.md](./spec-02-library-dashboard.md)  
   游戏库、导入/去重/刮削/启动闭环和统一活动仪表盘。
6. [spec-03-ai-assistant.md](./spec-03-ai-assistant.md)  
   AI Gateway、Secret、结构化输出、库整理、自然搜索、推荐和降级。
7. [spec-04-ui-design-system.md](./spec-04-ui-design-system.md)  
   UI 审计、设计令牌、四主入口一致性、Big Picture、动效、A11y 与截图矩阵。
8. [PLANNING-AUDIT.md](./PLANNING-AUDIT.md)  
   用户要求到规划证据的逐项映射与当前质量基线。

## 实施进度

[IMPLEMENTATION-STATUS.md](./IMPLEMENTATION-STATUS.md) 记录各批次、代理和验证证据。

## 必须先关掉的 P0

在新功能 PR 合并前，先完成：

- command 注册 / `build.rs` / capability / 前端 CommandMap 单一真源；当前 3 个已调用命令缺 capability。
- 数据库 migration 失败不得删除唯一主库或静默切内存库；增加事务、备份和只读 recovery mode。
- AI/Steam/Bangumi/PicACG 等 Secret 迁移出 localStorage/明文 settings JSON。
- 版本号与 tag/安装包/updater JSON 一致。
- Provider/Activity/Progress/Health/Job 契约冻结。

## 推荐执行批次

### Batch 0 — Baseline & Safety

A0 + A1 + A9：版本、command contract、DB recovery、SecretStore、测试证据。

### Batch 1 — Shared Foundation

A1 + A7：Provider/Activity/Job contracts、UI shell/state boundary。

### Batch 2 — Four Pillars

A2/A3/A4/A5 并行：游戏库、记录、番剧、漫画；feature flag 隔离。

### Batch 3 — Cross-cutting

A6/A7/A8：AI、整体 UI、大屏、下载/备份/诊断。

### Batch 4 — Integration & Release

A0 + A9：迁移、性能、真实源、签名安装升级、回滚和发布。

## 当前质量基线

2026-07-10 本地验证：

- `npm run check`：0 errors / 0 warnings。
- `npm run test:unit`：155 passed / 1 skipped。
- `npm run test:visual`：3 passed。
- `cargo fmt --check`：通过。
- `cargo clippy -- -D warnings`：通过。
- `cargo test`：126 passed，2 ignored（环境/live）。

这些结果证明当前代码可作为规划基线，但不证明 0.12.1 目标已完成。



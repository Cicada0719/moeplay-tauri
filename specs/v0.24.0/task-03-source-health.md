# Task 03：源健康与规则更新错误分类

## 目标与依赖

依赖 Task 02 的播放结果契约。让健康检查说明失败阶段，规则更新失败时保留上一版本，并修复 CI 误报。

## 允许修改

`src-tauri/src/rules/` 的更新与健康模块、规则 commands/API、源状态 UI、`.github/workflows/rules-health.yml` 及相关测试。禁止修改播放共享文件、漫画/小说、发布产物。

## 接口

保留现有 `rules_check_and_update`、`rules_probe_health`、`rules_get_health`；结果增加 `stage`、`errorKind`（network/http/tls-dns/timeout/challenge/script/empty/cancelled/unknown）、可选 `httpStatus`、`checkedAt`、`lastKnown`。

## 行为

- HTTP 非 2xx、错误页面、无效选择器、空结果和超时必须独立分类。
- 无记录或超过 24 小时为 `unknown`；连续 1–2 次失败为 `degraded`，连续 3 次为 `abnormal`；失败保留旧结果。
- 规则下载先暂存，manifest、签名、哈希和结构校验全部通过后原子替换；失败继续使用缓存。
- 同名自定义规则拥有稳定 ID，不覆盖内置规则。
- CI 构建失败、报告缺失、真实探测失败分别输出；失败报告和日志始终上传；告警复用同一开放 Issue，补足最小权限。

## 验收

篡改包、断网、403/500、空结果、选择器错误、超时和报告缺失都有不同错误类型；上一份规则仍可用；健康 UI 不宣称外部源永久可用；规则单测和 workflow dry-run 通过。


# SPEC-03：AI 助手产品化

## 1. 现状证据

- `src-tauri/src/scraper/ai.rs` 已有 OpenAI-compatible Chat Completions 调用。
- `src-tauri/src/scraper/ai_presets.rs` 已有 OpenAI、DeepSeek、Ollama、Claude、自定义 Provider 和多个 preset。
- `Settings`/前端类型直接包含 `ai_api_key`、`ai_api_url`、`ai_model`，设置页可编辑明文 Key。
- AI 主要服务刮削简介、标签、翻译、图片关键词、分级、标题识别；缺少统一任务状态、schema validation、预算、审计与面向用户的助手入口。
- 当前默认模型名和 API URL 是具体厂商值，容易把产品能力与单一 Provider 绑定。
- AI 与非 AI 刮削混在 command/service 中，错误时只作为增强失败处理，难以观察成本和质量。

### 1.1 代码复核补充（P0）

- **Secret 暴露面比“设置页明文输入”更大**：`get_settings`/`update_settings` 会把完整 `Settings`（含 `ai_api_key`）送入前端全局 store；`get_ai_providers` 还会把 Key 填入 `AiProvider.api_key` 后直接返回；`export_database` 导出的 `AppDatabase.settings` 同样包含 Key。迁移必须同时切断 IPC DTO、数据库导出/导入和诊断日志路径，不能只改输入框。
- **Endpoint 与 Secret 必须绑定**：当前用户可编辑任意 `ai_api_url`，后端会把同一 Key 作为 Bearer Token 发往该地址。新设计必须校验协议/URL、禁止 URL 内凭据、远端默认要求 HTTPS、仅对显式本地 Provider 放行 loopback HTTP；地址 origin 变化后必须重新确认/重新绑定 Secret，禁止静默复用。
- **现有多 Provider 只是声明层**：内置列表有 OpenAI、DeepSeek、Ollama、Claude，但六个 preset 全部绑定 `openai`；`run_ai_preset` 又统一覆盖为单组 Settings。Ollama 因“Key 非空”门槛实际上不可无 Key 使用；Claude 仅增加 header，仍复用 OpenAI 请求体；`Custom` 只有枚举、无持久化/UI。v0.12.1 必须保留现有 provider/preset ID 做迁移，但未通过 contract test 的 Provider 不得在 UI 标记为可用。
- **结构化输出与降级尚未闭环**：`scraper/ai.rs` 只有启发式 JSON 提取，`run_ai_preset` 返回原始字符串，缺少 schema/业务校验；错误会携带完整响应 body，且 `tracing::warn!(error=%e)` 可能把它写入日志。当前刮削已有“AI 失败保留原结果/本地封面”降级，这是应保留的基线；但没有 Retry-After、预算、真实取消或 Provider 链。跨 Provider 回退必须显式开启，且不得从本地静默切到远端。
- **现有任务队列不能直接充当 AI Orchestrator**：`TaskQueue.cancel` 只改状态，不会中止 HTTP；任务状态还能由前端 command 更新。AI 任务必须由后端持有 cancellation token、并发信号量和终态写入权，迟到响应不得写库。

## 2. 目标

1. 建立 Provider-agnostic AI Gateway，远端兼容 API、本地 Ollama/本地服务均可使用。
2. API Key 不进入普通设置、前端 state、导出或日志。
3. 所有 AI 输出使用版本化 schema 验证，非法输出不会写库。
4. AI 是可关闭的增益层；关闭、限流、离线、余额不足时主功能仍正常。
5. 先交付三个高价值场景：库整理建议、自然语言搜索、个性化“玩什么/看什么”。
6. 所有写操作以 change set 预览、确认和 undo 交付。

## 3. 非目标

- 不实现能够任意调用系统命令、读写文件或操纵账号的自治 Agent。
- 不上传存档、完整本地路径、Token、Cookie 或成人内容历史，除非用户对单次任务明确授权。
- 不允许 AI 自动删除、移动、重命名游戏或覆盖元数据。
- 不将推荐结果宣称为客观结论；必须展示推荐理由和数据依据。
- 不承诺兼容所有模型方言；先稳定 OpenAI-compatible + Ollama。

## 4. 产品场景

### 4.1 库整理 Copilot

输入：规范化后的游戏名、现有 metadata、来源候选、重复信号，不包含不必要路径。

输出：

```ts
interface LibraryChangeSet {
  summary: string;
  confidence: number;
  operations: Array<
    | { type: "set_field"; gameId: string; field: string; value: unknown; reason: string }
    | { type: "add_tag"; gameId: string; tag: string; reason: string }
    | { type: "possible_duplicate"; gameIds: string[]; reason: string }
    | { type: "needs_review"; gameId: string; reason: string }
  >;
}
```

UI 展示逐条 diff；默认全不勾选或只勾选高置信、无破坏操作。应用后生成 undo record。

### 4.2 自然语言搜索

用户输入：“找最近没玩过、全年龄、轻松、十小时以内的作品”。

AI 只把自然语言编译为本地过滤 DSL：

```json
{
  "kind": "game",
  "filters": [
    {"field":"lastPlayedAt","op":"is_null"},
    {"field":"contentRating","op":"eq","value":"all_ages"},
    {"field":"tags","op":"contains_any","value":["轻松","治愈"]},
    {"field":"estimatedHours","op":"lte","value":10}
  ],
  "sort":[{"field":"userAffinity","direction":"desc"}],
  "explanation":"..."
}
```

DSL 必须经过白名单 parser；不允许 AI 生成 SQL。若模型不可用，仍提供关键词/结构化筛选。

### 4.3 “玩什么 / 看什么”

推荐引擎先由本地规则召回：收藏、未完成、最近类别、可用源、时间预算、显式排除。LLM 只负责 rerank 或解释，不能凭空引入库外资源。

每条推荐展示：

- 为什么推荐；
- 使用了哪些本地信号；
- 预计耗时/可用集数；
- 可直接执行的继续/播放/阅读操作；
- “不感兴趣/少推荐此类”反馈。

### 4.4 摘要、翻译、标签和笔记

- 只对已有可见文本操作。
- 原文与 AI 版本并存。
- 标签写入 `source=ai`、模型、prompt version、时间和置信度。
- 笔记建议插入草稿，不自动覆盖用户笔记。

### 4.5 AI 状态中心

设置页只配置 Provider；任务历史放在 AI 状态中心：queued/running/succeeded/failed/cancelled、耗时、估算 token、错误分类。默认不保存完整 prompt/response，只保存脱敏摘要和 schema version。

## 5. 架构

```text
UI experiences
  ├─ library cleanup
  ├─ natural language search
  ├─ recommendation explanation
  └─ metadata/note helpers
       ↓
AI Orchestrator (task, budget, cancellation, context policy)
       ↓
Prompt Registry + Output Schema Registry
       ↓
AI Gateway
  ├─ OpenAICompatibleProvider
  ├─ OllamaProvider
  └─ MockProvider
       ↓
SecretStore + HTTP client + telemetry(redacted)
```

### 5.1 Provider contract

```rust
#[async_trait]
pub trait AiProvider {
    fn capabilities(&self) -> AiCapabilities;
    async fn health(&self) -> Result<AiHealth, AiError>;
    async fn generate_structured(
        &self,
        request: StructuredRequest,
        cancel: CancellationToken,
    ) -> Result<StructuredResponse, AiError>;
}
```

Capabilities：`structured_output`、`json_mode`、`streaming`、`vision`、`local`、`max_context_tokens`。

### 5.2 Prompt registry

每个 prompt 定义：

- `id`、`version`、`useCase`；
- system template 和最小上下文；
- input/output JSON schema；
- 默认模型 capability；
- max tokens、timeout、retry policy；
- privacy fields；
- fixtures 与 evaluation cases。

Prompt 不散落在 command 或 UI 中。

### 5.3 Output validation

处理顺序：

1. 限制 response body 大小。
2. 提取 JSON（兼容 fenced block）。
3. JSON parse。
4. schema validation。
5. 业务 validation（ID 必须来自输入、field 白名单、操作数量限制）。
6. 生成 preview change set。

任何一步失败都返回标准错误，不降级为“尽量写入”。

## 6. Secret 与隐私

### 6.1 SecretStore

- Windows 正式环境：Credential Manager/keyring。
- Settings 只保存 `providerId`、`secretRef`、`configured`，不保存明文。
- 前端 `get_ai_provider_config` 返回 masked metadata，不返回 Key。
- 导出设置时跳过 secret；恢复后要求重新输入或选择是否导入加密凭据。
- 测试使用内存 SecretStore。

### 6.2 Context policy

每个 use case 定义字段白名单：

- 可发送：标题、公开简介、标签、用户主动选中的笔记片段。
- 默认禁止：绝对路径、用户名、存档内容、账号 ID、Token、Cookie、完整成人阅读历史。
- UI 在首次调用和 provider 变更时显示“将发送的数据类型”。
- 本地 Provider 标识为“数据不离开本机”，但仍执行最小化上下文。

### 6.3 日志

- 只记录 task ID、provider、model、prompt version、duration、token estimate、error kind。
- URL 日志去掉 query；header 永不记录 Authorization/x-api-key。
- panic/diagnostics bundle 运行 redaction scanner。

## 7. 预算、速率和降级

Settings：

- 月预算/软提醒；
- 单任务最大 token、超时；
- 最大并发；
- 是否允许自动重试；
- 优先本地/优先低成本/手动选择策略。

错误分类：`NotConfigured`、`Auth`、`RateLimited`、`BudgetExceeded`、`Timeout`、`InvalidOutput`、`ProviderUnavailable`、`Cancelled`、`PolicyRejected`。

降级：

- 自然语言搜索失败 → 关键词搜索 + 显示结构化筛选。
- 推荐失败 → 本地规则排序。
- 刮削增强失败 → 原始来源数据。
- 摘要/标签失败 → 保持原数据，不产生空覆盖。

## 8. 数据模型

```ts
interface AiProviderConfig {
  id: string;
  kind: "openai-compatible" | "ollama";
  displayName: string;
  baseUrl: string;
  model: string;
  secretConfigured: boolean;
  capabilities: AiCapabilities;
  enabled: boolean;
}

interface AiTaskRecord {
  id: string;
  useCase: string;
  providerId: string;
  model: string;
  promptVersion: string;
  status: "queued" | "running" | "succeeded" | "failed" | "cancelled";
  createdAt: string;
  completedAt?: string;
  inputSummary: Record<string, number | string | boolean>;
  outputSchema: string;
  tokenEstimate?: number;
  errorKind?: string;
}

interface AiChangeSet {
  id: string;
  taskId: string;
  operations: AiOperation[];
  appliedAt?: string;
  undoToken?: string;
}
```

SQLite：`ai_provider_configs`（无 secret）、`ai_tasks`、`ai_change_sets`、`ai_feedback`。任务正文默认不持久化。

## 9. API / Commands

- `ai_list_providers`
- `ai_save_provider_config`
- `ai_set_provider_secret`
- `ai_delete_provider_secret`
- `ai_test_provider`
- `ai_list_use_cases`
- `ai_run_task`
- `ai_cancel_task`
- `ai_get_task`
- `ai_preview_change_set`
- `ai_apply_change_set`
- `ai_undo_change_set`
- `ai_compile_search`
- `ai_recommend`
- `ai_submit_feedback`

所有 command 只传 task/result DTO；前端不直接拼 Authorization request。

## 10. 文件级改动

### 新 Rust 模块

- `src-tauri/src/ai/mod.rs`
- `src-tauri/src/ai/provider.rs`
- `src-tauri/src/ai/openai_compatible.rs`
- `src-tauri/src/ai/ollama.rs`
- `src-tauri/src/ai/orchestrator.rs`
- `src-tauri/src/ai/prompts.rs`
- `src-tauri/src/ai/schema.rs`
- `src-tauri/src/ai/context_policy.rs`
- `src-tauri/src/ai/redaction.rs`
- `src-tauri/src/ai/change_set.rs`

现有 `scraper/ai.rs` 和 `ai_presets.rs` 先改为 adapter，稳定后删除重复调用路径。

### 新前端模块

- `src/lib/features/ai/aiStore.svelte.ts`
- `src/lib/features/ai/components/ProviderSettings.svelte`
- `src/lib/features/ai/components/AiTaskCenter.svelte`
- `src/lib/features/ai/components/ChangeSetPreview.svelte`
- `src/lib/features/ai/components/NaturalSearchInput.svelte`
- `src/lib/features/ai/components/RecommendationReasons.svelte`

SettingsPage 只嵌入 ProviderSettings，不再 bind 明文 `ai_api_key`。

## 11. 任务拆分

### AI-0 P0 Secret 止血（先于其他任务）

- 引入不含 Secret 的 `PublicSettings`/masked provider DTO，立即禁止 `get_settings`、`update_settings`、`get_ai_providers` 返回 Key。
- `export_database`、导入兼容层、诊断与日志加入 Secret scrub/redaction；Provider 错误 body 限长并分类，不向前端透传原文。
- 为旧 `ai_api_key` 实施“写入 SecretStore → 回读校验 → 清空 SQLite 明文字段”的一次性迁移；迁移失败时也不得经 IPC 回显明文。
- 收紧 Tauri capability：移除生产环境通用 raw-prompt command；Secret 写入、Provider 测试和 AI 执行只走用途受限 command。

### AI-1 Gateway / Provider

- OpenAI-compatible、Ollama、Mock。
- health、capabilities、timeout/cancel、redacted errors。

### AI-2 Secret / Config Migration

- SecretStore。
- 从旧 `ai_api_key` 一次性迁移；迁移成功后清空明文字段。
- 验收：旧 settings 导出中不再含 key。

### AI-3 Prompt / Schema Registry

- 迁移六个已有 preset。
- 每个 use case 具备 fixture、schema 和 business validator。

### AI-4 Task / Budget

- BackgroundJob 集成、并发、预算、token estimate、历史。

### AI-5 Library Cleanup

- change set、preview、apply、undo。

### AI-6 Natural Search

- 白名单 DSL、parser、解释 UI、无 AI fallback。

### AI-7 Recommendation

- 本地召回、LLM 可选解释、feedback。

### AI-8 Metadata / Notes

- 摘要、翻译、标签、笔记草稿；field provenance。

## 12. 验收标准

- Key 不存在于前端对象、普通 settings JSON、诊断包和日志。
- Mock provider 下所有 use case 输出通过 schema；非法 JSON/越权 ID 被拒绝。
- AI 关闭/无网络/429/超时/预算耗尽时四主功能可正常使用。
- Library change set 在用户确认前 DB 零变化；apply 后可以 undo。
- 自然语言搜索只生成白名单 DSL，不生成 SQL/脚本。
- 推荐只引用真实存在且当前可用的 resource ID。
- 任务可取消；取消后迟到响应不能更新 UI 或写库。
- Provider health test 能区分 auth、rate limit、timeout、invalid response。
- 使用哨兵 Key 的自动化测试证明：Settings IPC、Provider DTO、数据库导出、诊断包、日志和错误字符串均不包含该值。
- 修改 Provider endpoint origin 后旧 Secret 不会被自动发送；远端 HTTP、带凭据 URL、非 http(s) scheme 被拒绝，本地 loopback HTTP 仅对本地 Provider 放行。
- 无 Key 的 Ollama contract test 可完成至少一个结构化用例且无远端网络请求；OpenAI-compatible/DeepSeek 通过统一 contract；Claude 若无独立 adapter contract 则隐藏或标记不可用。
- 六个既有 preset ID 完成版本化迁移；每个结果均通过 schema 与业务校验，非法输出最多允许一次受预算约束的修复重试，仍失败则不写库。
- AI 取消会实际 abort 请求；跨 Provider 回退默认关闭，且任何本地→远端切换都需用户显式授权。

## 13. 测试矩阵

| 测试 | 覆盖 |
|---|---|
| provider contract | OpenAI-compatible/Ollama/Mock |
| secret migration | old plaintext → keyring ref → plaintext removed |
| redaction | auth headers, query token, path, diagnostics |
| output schema | valid/fenced/invalid/oversized/wrong IDs |
| budget | soft warning/hard stop/concurrency |
| cancellation | before send/during request/late response |
| change set | preview/apply/undo/conflict |
| search DSL | whitelist/unknown field/type mismatch |
| recommendation | no hallucinated IDs, excluded item respected |
| offline fallback | all four experiences |
| visual | provider settings/task center/change preview |

## 14. 风险与回滚

- **Provider 方言差异**：capability detect；不支持 structured output 时使用 JSON prompt + validator。
- **Key 迁移失败**：保留旧字段直到 keyring write+read 验证成功；失败只提示，不删除。
- **AI 成本不可控**：硬预算、max tokens、并发 1 默认、批量任务二次确认。
- **幻觉/越权操作**：ID/field 白名单、change set、人工确认、undo。
- **本地模型性能差**：health benchmark，允许选择小模型，任务超时。
- **AI UI 侵占主流程**：AI 入口作为辅助，不新增常驻弹窗或强制 onboarding。
- feature flag：`ai_gateway_v2`；旧 AI 刮削 adapter 保留到 RC 验证完成。

## 15. 里程碑

- M1（2 天）：Gateway、SecretStore、配置迁移、Mock tests。
- M2（2 天）：Prompt/schema/task/budget，迁移现有 preset。
- M3（2 天）：库整理与自然语言搜索。
- M4（2 天）：推荐、摘要/笔记、UI、离线/安全验收。

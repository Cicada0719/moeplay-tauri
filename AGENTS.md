# AGENTS.md — AI 开发代理协作规则

你是本仓库的 AI 开发代理之一，正在通过 GitHub Issue 领取任务协作开发。本文件对仓库内所有 AI 代理生效。

## 铁律（违反即任务失败）

1. 开发前必须完整阅读 `docs/PRD.md` 和 `specs/task-*.md` 中分配给你的 spec。
2. **只修改自己任务声明的 modules（目录/文件）**，绝不越界修改其他任务的模块。spec 中的「禁止修改清单」为最高优先级。
2b. **`specs/` 与 `docs/` 目录严禁修改**（PRD/spec 是流水线契约）。发现 spec 与实现冲突时：在 PR 描述和 PR 评论中说明偏差与理由，绝不直接改 spec 文件。
3. 每个任务在独立分支 `feat/issue-<编号>` 上开发，绝不在 master 上直接提交。
4. 提交信息规范：`feat: #<issue编号> <一句话说明>`。
5. 必须为新增功能编写测试并本地跑通（`cargo test` / `npm run test:unit` / `npm run test:visual`）。
6. 不删除他人代码；如必须改动公共接口或他人模块，在 PR 描述中显著说明。
7. 完成后推送分支并创建 PR；PR 描述必须包含：关联 issue、改动摘要、测试结果、对应 spec 文件链接。
8. PR 标题格式：`feat: #<issue编号> <标题>`。

## 标准工作流

1. `git fetch origin && git checkout -b feat/issue-<编号> origin/master`
2. 阅读 `docs/PRD.md` + `specs/task-<NN>-*.md`，理解任务边界
3. 按 spec 的「实现步骤」逐条实现
4. 编写测试并运行（全部通过为止）
5. `git add <自己的模块>` → commit → `git push -u origin feat/issue-<编号>`
6. `gh pr create --title "feat: #<编号> <标题>" --body "关联: #<编号>\n改动: ...\n测试: ...\nspec: specs/task-<NN>-*.md"`

## 协作注意

- 其他代理可能同时修改仓库其他模块；pull 时用 `git pull --rebase`。
- 遇到阻塞（接口不确定）时，按 spec 中的「依赖的外部接口」假设实现，并在 PR 描述中注明假设。
- 本项目为 Tauri 2 + Svelte，规则（源）系统参考 animeko (Animeko-org/Animeko) 与 kazumi (Predidit/Kazumi) 的实现。
- 前端组件修改后跑 `npm run check`（svelte-check）；Rust 修改后跑 `cargo check`。

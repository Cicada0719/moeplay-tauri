export const AI_COMMANDS = {
  providerStatus: "ai_v2_provider_status",
  budgetStatus: "ai_v2_budget_status",
  startStructuredTask: "ai_v2_start_structured_task",
  taskStatus: "ai_v2_task_status",
  taskResult: "ai_v2_task_result",
  cancelTask: "ai_v2_cancel_task",
  previewChangeSet: "ai_changes_preview",
  applyChangeSet: "ai_changes_apply",
  undoChangeSet: "ai_changes_undo",
} as const;

export type AiCommandName = (typeof AI_COMMANDS)[keyof typeof AI_COMMANDS];

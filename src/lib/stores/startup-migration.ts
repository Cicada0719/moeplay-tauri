// 启动模式一次性迁移判定（纯函数，便于在 vitest 下测试，不依赖 runes）。
//
// 背景：历史默认启动模式是 "dashboard"（仅最大化、保留任务栏）；
// 新默认是 "fullscreen"。为了让历史用户也能享受全屏默认，需要做一次迁移；
// 但**只能迁移一次**——否则会反复把用户主动选择的 "普通模式(dashboard)" 改写回去，
// 导致"普通模式永远存不住"（用户反馈的 bug）。

export const STARTUP_MIGRATED_KEY = "moegame-startup-migrated-v1";

/**
 * 是否需要把当前存储的启动模式迁移到 "fullscreen"。
 * 仅当：从未迁移过(`alreadyMigrated=false`) 且 当前存的是历史默认 "dashboard" 时为真。
 * 其余一律返回 false——尊重用户的任何主动选择。
 */
export function shouldMigrateStartupMode(
  storedMode: string | undefined,
  alreadyMigrated: boolean,
): boolean {
  return !alreadyMigrated && storedMode === "dashboard";
}

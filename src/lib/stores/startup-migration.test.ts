import { describe, expect, it } from "vitest";
import { shouldMigrateStartupMode } from "./startup-migration";

describe("startup mode migration guard", () => {
  it("migrates a legacy dashboard user exactly once", () => {
    // 从未迁移过 + 存的是历史默认 dashboard → 需要迁移
    expect(shouldMigrateStartupMode("dashboard", false)).toBe(true);
  });

  it("never re-writes after the one-time migration ran", () => {
    // 已迁移过 → 即使现在存的是 dashboard（用户主动选的"普通模式"）也不改写
    expect(shouldMigrateStartupMode("dashboard", true)).toBe(false);
  });

  it("respects a deliberate non-dashboard choice", () => {
    expect(shouldMigrateStartupMode("fullscreen", false)).toBe(false);
    expect(shouldMigrateStartupMode("big-picture", false)).toBe(false);
    expect(shouldMigrateStartupMode("big-picture", true)).toBe(false);
  });

  it("treats undefined stored mode as nothing to migrate", () => {
    expect(shouldMigrateStartupMode(undefined, false)).toBe(false);
  });
});

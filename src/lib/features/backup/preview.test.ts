import { describe, expect, it } from "vitest";
import { summarizeSnapshotDiff } from "./preview";

describe("save restore preview", () => {
  it("separates changed files and flags destructive replacement", () => {
    expect(summarizeSnapshotDiff({ added: ["new.sav"], removed: ["old.sav"], changed: ["slot1.sav"], unchanged: 4 })).toEqual({
      changedFiles: 3,
      totalCompared: 7,
      destructive: true,
    });
  });

  it("treats an additive-only restore as non-destructive", () => {
    expect(summarizeSnapshotDiff({ added: ["new.sav"], removed: [], changed: [], unchanged: 1 }).destructive).toBe(false);
  });
});

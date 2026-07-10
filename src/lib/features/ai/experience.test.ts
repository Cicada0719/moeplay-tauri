import { describe, expect, it } from "vitest";
import { GenerationGuard } from "./generation";
import { validateChangeSetPreview, validateFilterDslResult, validateRecommendationExplanations } from "./validation";

const validPreview = {
  id: "change-1",
  taskId: "task-1",
  summary: "补全资料",
  confidence: 0.9,
  state: "awaiting_confirmation",
  operations: [{
    selected: true,
    operation: { type: "add_tag", gameId: "game-1", tag: "治愈", reason: "本地元数据来源一致" },
  }],
};

describe("AI frontend validation guards", () => {
  it("rejects executable or non-whitelisted filter clauses", () => {
    const result = validateFilterDslResult({
      kind: "game",
      filters: [{ field: "rawSql", op: "execute", value: "DELETE FROM games" }],
      sort: [],
      explanation: "unsafe",
    }, "game");
    expect(result.ok).toBe(false);
  });

  it("normalizes every preview operation to unselected", () => {
    const result = validateChangeSetPreview(validPreview);
    expect(result.ok).toBe(true);
    if (result.ok) expect(result.value.operations.map((entry) => entry.selected)).toEqual([false]);
  });

  it("rejects recommendation explanations outside the supplied candidate IDs", () => {
    const result = validateRecommendationExplanations([
      { resourceId: "not-a-candidate", explanation: "invented" },
    ], ["game-1"]);
    expect(result.ok).toBe(false);
  });

  it("invalidates earlier generations after cancel or replacement", () => {
    const guard = new GenerationGuard();
    const first = guard.begin();
    const second = guard.begin();
    expect(first.signal.aborted).toBe(true);
    expect(guard.isCurrent(first.generation)).toBe(false);
    expect(guard.isCurrent(second.generation)).toBe(true);
    guard.cancel();
    expect(guard.isCurrent(second.generation)).toBe(false);
  });
});

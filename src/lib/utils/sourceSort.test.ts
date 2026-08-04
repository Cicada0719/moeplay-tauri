// sortByHealth 纯函数测试（spec §6.2 测试 16）
import { describe, expect, it } from "vitest";
import { sortByHealth } from "./sourceSort";
import type { SourceHealthInfo } from "../api/rules";

function health(
  sourceId: string,
  status: SourceHealthInfo["status"],
  lastLatencyMs: number | null = null,
): SourceHealthInfo {
  return {
    sourceId,
    status,
    consecutiveFailures: 0,
    lastCheckedAt: 1,
    lastLatencyMs,
    lastError: null,
  };
}

describe("sortByHealth", () => {
  it("混合状态：Healthy 置顶、Abnormal 沉底、同状态按延迟排序", () => {
    const sources = [{ id: "a" }, { id: "b" }, { id: "c" }, { id: "d" }, { id: "e" }];
    const list = [
      health("d", "Abnormal", 300),
      health("b", "Healthy", 200),
      health("e", "Degraded", 100),
      health("c", "Unknown"),
      health("a", "Healthy", 50),
    ];
    expect(sortByHealth(sources, list).map((s) => s.id)).toEqual([
      "a",
      "b",
      "c",
      "e",
      "d",
    ]);
  });

  it("空 health 数组：全部 Unknown，保持原序稳定", () => {
    const sources = [{ id: "x" }, { id: "y" }, { id: "z" }];
    expect(sortByHealth(sources, []).map((s) => s.id)).toEqual(["x", "y", "z"]);
  });

  it("同状态按 lastLatencyMs 升序；无延迟数据的排后", () => {
    const sources = [{ id: "a" }, { id: "b" }, { id: "c" }, { id: "d" }];
    const list = [
      health("b", "Healthy", 100),
      health("c", "Healthy", null),
      health("d", "Healthy", 50),
      health("a", "Healthy", 10),
    ];
    expect(sortByHealth(sources, list).map((s) => s.id)).toEqual(["a", "d", "b", "c"]);
  });

  it("不修改入参数组（纯函数）", () => {
    const sources = [{ id: "a" }, { id: "b" }];
    const original = [...sources];
    sortByHealth(sources, [health("b", "Abnormal")]);
    expect(sources).toEqual(original);
  });
});

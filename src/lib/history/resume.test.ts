// resume.ts 续读目标解析单测（spec §4 步骤 7 契约）
import { describe, expect, it } from "vitest";
import { resolveResumeTarget } from "./resume";
import type { HistoryItem } from "./types";

function baseItem(overrides: Partial<HistoryItem>): HistoryItem {
  return {
    id: "id",
    contentId: "c1",
    contentType: "manga",
    title: "标题",
    cover: null,
    sourceId: "src",
    chapterId: "ch1",
    chapterTitle: "第1话",
    pageIndex: 7,
    positionSec: 0,
    scrollPct: 0,
    progress: 7,
    updatedAt: Date.now(),
    deviceId: "dev",
    deleted: false,
    ...overrides,
  };
}

describe("resolveResumeTarget", () => {
  it("manga → 携带单页原子 pageIndex（不因渲染模式偏移）", () => {
    const target = resolveResumeTarget(baseItem({ contentType: "manga", pageIndex: 7 }));
    expect(target.kind).toBe("manga");
    if (target.kind !== "manga") return;
    expect(target.pageIndex).toBe(7);
  });

  it("manga pageIndex 越界/负值被夹取到 ≥0", () => {
    const target = resolveResumeTarget(baseItem({ contentType: "manga", pageIndex: -3 }));
    if (target.kind !== "manga") return;
    expect(target.pageIndex).toBe(0);
  });

  it("novel → 携带 0~1 scrollPct（夹取）", () => {
    const target = resolveResumeTarget(baseItem({ contentType: "novel", scrollPct: 0.45 }));
    expect(target.kind).toBe("novel");
    if (target.kind !== "novel") return;
    expect(target.scrollPct).toBeCloseTo(0.45, 5);
  });

  it("novel scrollPct 越界被夹取到 0~1", () => {
    const target = resolveResumeTarget(baseItem({ contentType: "novel", scrollPct: 3.2 }));
    if (target.kind !== "novel") return;
    expect(target.scrollPct).toBe(1);
  });

  it("anime → 携带 positionSec 占位（本任务不实现番剧续播）", () => {
    const target = resolveResumeTarget(
      baseItem({ contentType: "anime", positionSec: 123.5, pageIndex: 0 }),
    );
    expect(target.kind).toBe("anime");
    if (target.kind !== "anime") return;
    expect(target.positionSec).toBeCloseTo(123.5, 5);
  });
});

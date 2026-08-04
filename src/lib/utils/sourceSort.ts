// 源健康状态排序工具（spec task-02 §3.3 / §6.2 测试 16）
//
// 规则：状态权重 `Healthy=0 < Unknown=1 < Degraded=2 < Abnormal=3`，权重升序；
// 同权重按 `lastLatencyMs` 升序（无延迟数据的排后）；**Abnormal 自然沉底**（FR-04）。

import type { SourceHealthInfo } from "../api/rules";

/** 状态权重（Abnormal 最大 → 排序时沉底）。 */
export const HEALTH_WEIGHT: Record<SourceHealthInfo["status"], number> = {
  Healthy: 0,
  Unknown: 1,
  Degraded: 2,
  Abnormal: 3,
};

/**
 * 按健康状态对源列表排序（纯函数，不修改入参）：
 * - 无健康记录（含 `health` 为空数组）→ 全部按 `Unknown` 处理，保持原序稳定；
 * - 同状态按 `lastLatencyMs` 升序，无延迟数据的排后。
 */
export function sortByHealth<T extends { id: string }>(
  sources: T[],
  health: SourceHealthInfo[],
): T[] {
  const byId = new Map(health.map((h) => [h.sourceId, h]));
  return [...sources].sort((a, b) => {
    const ha = byId.get(a.id);
    const hb = byId.get(b.id);
    const wa = HEALTH_WEIGHT[ha?.status ?? "Unknown"];
    const wb = HEALTH_WEIGHT[hb?.status ?? "Unknown"];
    if (wa !== wb) return wa - wb;

    const la = ha?.lastLatencyMs ?? null;
    const lb = hb?.lastLatencyMs ?? null;
    if (la == null && lb == null) return 0;
    if (la == null) return 1; // 无延迟数据的排后
    if (lb == null) return -1;
    return la - lb;
  });
}

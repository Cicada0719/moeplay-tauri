import type {
  MediaPresentationAction,
  MediaPresentationActionId,
  MediaPresentationItem,
  ScenePresentationEntry,
} from "./types";

export function statusLabel(status?: string): string {
  switch (status) {
    case "playing": return "正在游玩";
    case "completed": return "已完成";
    case "dropped": return "已搁置";
    case "on_hold": return "暂停中";
    case "plan_to_play": return "计划游玩";
    case "replaying": return "再次游玩";
    default: return "尚未开始";
  }
}

export function formatPlaytime(totalSeconds?: number): string {
  const seconds = Math.max(0, totalSeconds ?? 0);
  if (seconds < 3600) return seconds > 0 ? `${Math.max(1, Math.round(seconds / 60))} 分钟` : "尚无记录";
  const hours = seconds / 3600;
  return hours >= 100 ? `${Math.round(hours)} 小时` : `${hours.toFixed(hours < 10 ? 1 : 0)} 小时`;
}

export function findAction(
  item: MediaPresentationItem,
  actionId: MediaPresentationActionId,
): MediaPresentationAction | undefined {
  return item.actions.find((action) => action.id === actionId && action.enabled);
}

export function runAction(
  item: MediaPresentationItem,
  actionId: MediaPresentationActionId,
  delegate?: (item: MediaPresentationItem, action: MediaPresentationAction) => void | Promise<void>,
): void {
  const action = findAction(item, actionId);
  if (!action) return;
  void (delegate ? delegate(item, action) : action.run());
}

function recency(item: MediaPresentationItem): number {
  return Date.parse(item.metadata.lastPlayed || "") || 0;
}

export function sceneEntries(
  items: readonly MediaPresentationItem[],
  selectedId?: string | null,
  limit = 6,
): ScenePresentationEntry[] {
  const ordered = [...items].sort((a, b) => {
    if (a.id === selectedId) return -1;
    if (b.id === selectedId) return 1;
    return recency(b) - recency(a);
  });
  const queues = ordered.map((item) => ({
    item,
    assets: [...item.media].sort((a, b) => {
      const priority = { hero: 0, screenshot: 1, cover: 2 } as const;
      return priority[a.role] - priority[b.role];
    }),
  }));
  const entries: ScenePresentationEntry[] = [];
  const seenAssets = new Set<string>();

  // First pass keeps title diversity before filling with additional frames.
  for (const queue of queues) {
    const asset = queue.assets.shift() ?? null;
    if (asset) seenAssets.add(asset.id);
    entries.push({
      id: `${queue.item.id}:${asset?.id || "placeholder"}:0`,
      item: queue.item,
      asset,
      position: entries.length,
    });
    if (entries.length >= limit) return entries;
  }

  let added = true;
  while (entries.length < limit && added) {
    added = false;
    for (const queue of queues) {
      const asset = queue.assets.find((candidate) => !seenAssets.has(candidate.id));
      if (!asset) continue;
      seenAssets.add(asset.id);
      entries.push({
        id: `${queue.item.id}:${asset.id}:${entries.length}`,
        item: queue.item,
        asset,
        position: entries.length,
      });
      added = true;
      if (entries.length >= limit) break;
    }
  }

  return entries;
}

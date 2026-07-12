import type { MediaPresentationItem, PresentationAsset } from "../model";
import { normalizeMediaIdentity } from "./mediaIdentity";
import { compareContinueCandidates, compareFeaturedCandidates, qualityWeight, selectDefaultItem } from "./presentationRanking";
import type { GameSceneComposition, SceneSlot, SceneSlotRole } from "./types";

function orderedAssets(item: MediaPresentationItem): PresentationAsset[] {
  const priority = { hero: 0, screenshot: 1, cover: 2 } as const;
  return [...item.media].sort((a, b) => priority[a.role] - priority[b.role] || a.id.localeCompare(b.id));
}

function roleFor(asset: PresentationAsset | null, position: number): SceneSlotRole {
  if (position === 0) return "lead";
  if (!asset) return "transition";
  if (asset.aspect === "portrait") return "portrait";
  return position % 4 === 1 ? "wide" : "support";
}

export function composeGameScene(
  items: readonly MediaPresentationItem[],
  selectedId?: string | null,
  limit = 12,
): GameSceneComposition {
  const selected = items.find((item) => item.id === selectedId) ?? selectDefaultItem(items);
  if (!selected || limit <= 0) return { selectedItemId: selected?.id ?? null, entries: [], activeIndex: 0 };

  const candidateItems = [
    selected,
    ...items.filter((item) => item.id !== selected.id && item.mediaQuality !== "d")
      .sort((a, b) => compareContinueCandidates(a, b) || compareFeaturedCandidates(a, b))
      .slice(0, 7),
  ];
  const queues = candidateItems.map((item) => ({ item, assets: orderedAssets(item), used: 0 }));
  const entries: SceneSlot[] = [];
  const seen = new Set<string>();

  const push = (item: MediaPresentationItem, asset: PresentationAsset | null) => {
    const position = entries.length;
    entries.push({
      id: `${item.id}:${asset?.id ?? "placeholder"}:${position}`,
      role: roleFor(asset, position),
      ownerItemId: item.id,
      item,
      asset,
      position,
      score: qualityWeight(item) + (item.id === selected.id ? 40 : 0) - position,
    });
  };

  const take = (queue: (typeof queues)[number]): PresentationAsset | null => {
    while (queue.assets.length) {
      const asset = queue.assets.shift()!;
      const identity = normalizeMediaIdentity(asset.src);
      if (!identity || seen.has(identity)) continue;
      seen.add(identity);
      queue.used += 1;
      return asset;
    }
    return null;
  };

  for (let i = 0; i < 4 && entries.length < limit; i += 1) {
    const asset = take(queues[0]);
    if (!asset) break;
    push(selected, asset);
  }
  for (let i = 1; i < queues.length && entries.length < limit; i += 1) {
    const asset = take(queues[i]);
    if (asset) push(queues[i].item, asset);
  }
  let progressed = true;
  while (entries.length < limit && progressed) {
    progressed = false;
    for (let i = 1; i < queues.length && entries.length < limit; i += 1) {
      if (queues[i].used >= 2) continue;
      const asset = take(queues[i]);
      if (!asset) continue;
      push(queues[i].item, asset);
      progressed = true;
    }
  }
  if (entries.length === 0) push(selected, null);
  return { selectedItemId: selected.id, entries, activeIndex: 0 };
}

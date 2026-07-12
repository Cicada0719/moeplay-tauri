import type { MediaPresentationItem, PresentationAsset } from "../model";
import { normalizeMediaIdentity } from "./mediaIdentity";
import { compareContinueCandidates, compareFeaturedCandidates, featuredScore, hasEnabledAction, selectDefaultItem } from "./presentationRanking";
import type { GameVisualComposition, VisualMediaSlot, VisualSlotAction, VisualSlotRole } from "./types";

function uniqueAssets(assets: readonly (PresentationAsset | undefined)[]): PresentationAsset[] {
  const seen = new Set<string>();
  return assets.filter((asset): asset is PresentationAsset => {
    if (!asset?.src) return false;
    const identity = normalizeMediaIdentity(asset.src);
    if (!identity || seen.has(identity)) return false;
    seen.add(identity);
    return true;
  });
}

function preferredAsset(item: MediaPresentationItem): PresentationAsset | null {
  return uniqueAssets([item.hero, ...item.screenshots, item.cover])[0] ?? null;
}

function slot(
  role: VisualSlotRole,
  item: MediaPresentationItem | null,
  asset: PresentationAsset | null,
  label: string,
  action: VisualSlotAction,
  score = 0,
): VisualMediaSlot {
  return { id: `${role}:${item?.id ?? "empty"}:${asset?.id ?? "placeholder"}`, role, ownerItemId: item?.id ?? null, item, asset, label, action, score };
}

function actionForItem(item: MediaPresentationItem | null, preferred: "open" | "select"): VisualSlotAction {
  if (!item) return { type: "none" };
  if (preferred === "open" && hasEnabledAction(item, "open")) return { type: "open-item", itemId: item.id };
  return { type: "select-item", itemId: item.id };
}

export function composeGameVisual(
  items: readonly MediaPresentationItem[],
  selectedId?: string | null,
): GameVisualComposition {
  const selectedItem = items.find((item) => item.id === selectedId) ?? selectDefaultItem(items);
  if (!selectedItem) {
    const roles: VisualSlotRole[] = ["lead", "scene-a", "scene-b", "continue", "featured"];
    return { selectedItem: null, backgroundAsset: null, chromaAsset: null, slots: roles.map((role) => slot(role, null, null, role, { type: "none" })) };
  }

  const selectedAssets = uniqueAssets([selectedItem.hero, ...selectedItem.screenshots, selectedItem.cover]);
  const lead = selectedAssets[0] ?? null;
  const scenes = selectedAssets.filter((asset) => asset.id !== lead?.id).slice(0, 2);
  const others = items.filter((item) => item.id !== selectedItem.id);
  const continueItem = others
    .filter((item) => Boolean(item.metadata.lastPlayed))
    .sort(compareContinueCandidates)[0] ?? null;
  const featuredItem = others
    .filter((item) => item.id !== continueItem?.id)
    .sort(compareFeaturedCandidates)[0] ?? null;

  const slots: VisualMediaSlot[] = [
    slot("lead", selectedItem, lead, "当前游戏", actionForItem(selectedItem, "open"), 100),
    slot("scene-a", selectedItem, scenes[0] ?? null, "场景 01", scenes[0] ? { type: "open-media", itemId: selectedItem.id, assetId: scenes[0].id } : { type: "none" }, 80),
    slot("scene-b", selectedItem, scenes[1] ?? null, "场景 02", scenes[1] ? { type: "open-media", itemId: selectedItem.id, assetId: scenes[1].id } : { type: "none" }, 70),
    slot("continue", continueItem, continueItem ? preferredAsset(continueItem) : null, "继续游玩", actionForItem(continueItem, "select"), continueItem ? 60 : 0),
    slot("featured", featuredItem, featuredItem ? preferredAsset(featuredItem) : null, "精选", actionForItem(featuredItem, "select"), featuredItem ? featuredScore(featuredItem) : 0),
  ];

  return { selectedItem, backgroundAsset: lead, chromaAsset: lead, slots };
}


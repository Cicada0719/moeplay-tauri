import type { MediaPresentationItem, PresentationAsset } from "../model";

export type VisualSlotRole = "lead" | "scene-a" | "scene-b" | "continue" | "featured";
export type VisualSlotAction =
  | { type: "open-item"; itemId: string }
  | { type: "select-item"; itemId: string }
  | { type: "open-media"; itemId: string; assetId: string }
  | { type: "none" };

export interface VisualMediaSlot {
  id: string;
  role: VisualSlotRole;
  ownerItemId: string | null;
  item: MediaPresentationItem | null;
  asset: PresentationAsset | null;
  label: string;
  action: VisualSlotAction;
  score: number;
}

export interface GameVisualComposition {
  selectedItem: MediaPresentationItem | null;
  backgroundAsset: PresentationAsset | null;
  chromaAsset: PresentationAsset | null;
  slots: VisualMediaSlot[];
}

export type SceneSlotRole = "lead" | "wide" | "portrait" | "support" | "transition";

export interface SceneSlot {
  id: string;
  role: SceneSlotRole;
  ownerItemId: string | null;
  item: MediaPresentationItem | null;
  asset: PresentationAsset | null;
  position: number;
  score: number;
}

export interface GameSceneComposition {
  selectedItemId: string | null;
  entries: SceneSlot[];
  activeIndex: number;
}

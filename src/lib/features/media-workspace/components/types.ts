import type {
  ContentMode,
  MediaPresentationAction,
  MediaPresentationActionId,
  MediaPresentationItem,
  PresentationAsset,
} from "../model";

export type MediaWorkspaceMode = ContentMode;

export interface MediaWorkspaceViewActions {
  /** When omitted, the action's model-provided run() handler is invoked. */
  onAction?: (
    item: MediaPresentationItem,
    action: MediaPresentationAction,
  ) => void | Promise<void>;
  onImport?: () => void | Promise<void>;
}

export interface ScenePresentationEntry {
  id: string;
  item: MediaPresentationItem;
  asset: PresentationAsset | null;
  position: number;
}

export type {
  MediaPresentationAction,
  MediaPresentationActionId,
  MediaPresentationItem,
  PresentationAsset,
};

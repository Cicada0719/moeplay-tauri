import type { Snippet } from "svelte";

export type UiDensity = "compact" | "comfortable" | "couch";
export type ShellElement = "main" | "div" | "section" | "article";
export type ShellWidth = "full" | "content" | "narrow";
export type ContentGridElement = "div" | "ul" | "ol";
export type ContentGridGap = "sm" | "md" | "lg";
export type PanelSide = "right" | "left";
export type DrawerSide = PanelSide | "bottom";
export type PanelSize = "sm" | "md" | "lg";
export type MediaVariant = "poster" | "landscape" | "square";
export type ViewState =
  | "ready"
  | "loading"
  | "refreshing"
  | "empty"
  | "no-results"
  | "error"
  | "offline"
  | "stale"
  | "partial";

export type AriaLiveMode = "off" | "polite" | "assertive";

export interface AsyncAction {
  label: string;
  onSelect: (event: MouseEvent) => void;
  ariaLabel?: string;
  disabled?: boolean;
  loading?: boolean;
}

export type AsyncDetails = string | Snippet;

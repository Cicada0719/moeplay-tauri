import type { Snippet } from "svelte";
import type { GamepadHandlers } from "../../components/switch/useGamepad.svelte";

export type HandheldMediaKind = "anime" | "comic" | "novel";

/** Shared semantic actions exposed by the Android media shell. */
export type HandheldMediaAction =
  | "up"
  | "down"
  | "left"
  | "right"
  | "pageLeft"
  | "pageRight"
  | "activate"
  | "launch"
  | "favorite"
  | "filter"
  | "back"
  | "start";

export type NovelReadingMode = "paged" | "scroll";

export type HandheldChromeMode = "persistent" | "auto" | "minimal";
export type HandheldViewState = "loading" | "ready" | "partial" | "empty" | "error";
export type HandheldArtworkRole = "home" | "anime" | "comic" | "novel" | "empty";

export interface HandheldStateAction {
  label: string;
  run: () => void | Promise<void>;
  kind?: "primary" | "secondary";
}

export interface HandheldStateModel {
  state: HandheldViewState;
  title?: string;
  description?: string;
  primaryAction?: HandheldStateAction;
  secondaryAction?: HandheldStateAction;
}

export interface HandheldArtworkSource {
  role: HandheldArtworkRole;
  cover?: string | null;
  fallback?: string;
  alt?: string;
}

export interface HandheldMediaShellProps {
  kind: HandheldMediaKind;
  title: string;
  subtitle?: string;
  progress?: number;
  progressLabel?: string;
  chromeMode?: HandheldChromeMode;
  artwork?: HandheldArtworkSource;
  state?: HandheldStateModel;
  panelOpen?: boolean;
  keepChromeVisible?: boolean;
  onback?: () => void;
  children: Snippet;
  headerActions?: Snippet;
  overlay?: Snippet;
  footer?: Snippet;
  handlers?: GamepadHandlers;
}

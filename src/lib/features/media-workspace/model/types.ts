export const CONTENT_MODES = ["visual", "index", "scene"] as const;
export type ContentMode = (typeof CONTENT_MODES)[number];

export const CONTENT_MODULES = ["games", "anime", "comics"] as const;
export type ContentModule = (typeof CONTENT_MODULES)[number];

/** Matches the production wallpaper/shell surface vocabulary. */
export const APP_SURFACES = ["browse", "management", "immersive"] as const;
export type AppSurface = (typeof APP_SURFACES)[number];

export type MediaAssetRole = "cover" | "hero" | "screenshot";
export type MediaAssetQuality = "a" | "b" | "c" | "d";

export interface PresentationAsset {
  id: string;
  src: string;
  role: MediaAssetRole;
  alt: string;
  aspect: "portrait" | "landscape" | "unknown";
  focalPoint?: { x: number; y: number };
}

export interface MediaPresentationMetadata {
  developer?: string;
  publisher?: string;
  platform?: string;
  releaseYear?: number;
  completionStatus?: string;
  totalSeconds?: number;
  lastPlayed?: string;
  rating?: number;
  tags: string[];
}

export type MediaPresentationActionId =
  | "open"
  | "select"
  | "launch"
  | "toggle-favorite";

export interface MediaPresentationAction {
  id: MediaPresentationActionId;
  label: string;
  emphasis: "primary" | "secondary" | "quiet";
  enabled: boolean;
  active?: boolean;
  run: () => void | Promise<void>;
}

/**
 * Store-neutral view model consumed by Visual, Index and Scene renderers.
 * Business stores are adapted at the boundary and never need to know about UI modes.
 */
export interface MediaPresentationItem {
  id: string;
  module: ContentModule;
  title: string;
  originalTitle?: string;
  subtitle?: string;
  description?: string;
  cover?: PresentationAsset;
  hero?: PresentationAsset;
  screenshots: PresentationAsset[];
  media: PresentationAsset[];
  mediaQuality: MediaAssetQuality;
  favorite: boolean;
  installed: boolean;
  metadata: MediaPresentationMetadata;
  actions: MediaPresentationAction[];
}

export interface ModuleWorkspaceMemory {
  mode: ContentMode;
  selectedItemId: string | null;
  focusedItemId: string | null;
  scroll: { x: number; y: number };
}

export interface MediaWorkspaceSnapshot {
  activeModule: ContentModule;
  surface: AppSurface;
  modules: Record<ContentModule, ModuleWorkspaceMemory>;
}

export function isContentMode(value: unknown): value is ContentMode {
  return CONTENT_MODES.includes(value as ContentMode);
}

export function isContentModule(value: unknown): value is ContentModule {
  return CONTENT_MODULES.includes(value as ContentModule);
}

export function isAppSurface(value: unknown): value is AppSurface {
  return APP_SURFACES.includes(value as AppSurface);
}

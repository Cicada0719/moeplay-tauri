export type ConceptTemplate = "cinematic" | "editorial" | "kinetic";
export type ContentMode = "visual" | "index" | "scene";
export type ContentModule = "games" | "anime" | "comics";
export type MotionQuality = "full" | "balanced" | "reduced";
export type MediaRatio = "wide" | "editorial" | "portrait" | "square" | "strip";
export type MediaShotType = "hero" | "scene" | "character" | "detail" | "cover" | "manga-page";
export type NavigationIntent =
  | "next" | "previous" | "page-next" | "page-previous"
  | "activate" | "back" | "switch-mode-left" | "switch-mode-right";

export interface ConceptMediaAsset {
  id: string;
  contentId: string;
  src: string;
  placeholder?: string;
  mediaType: "image" | "video";
  ratio: MediaRatio;
  shotType: MediaShotType;
  tone: "light" | "dark" | "mixed";
  dominantColor: string;
  focalPoint: { x: number; y: number };
  templateUsage: ConceptTemplate[];
  sourceUrl: string;
}

export interface ConceptContentItem {
  id: string;
  module: ContentModule;
  title: string;
  subtitle: string;
  description: string;
  status: string;
  progress: number;
  progressLabel: string;
  meta: string[];
  media: ConceptMediaAsset[];
}

export interface ContentStageState {
  template: ConceptTemplate;
  module: ContentModule;
  modeByModule: Record<ContentModule, ContentMode>;
  selectedIdByModule: Record<ContentModule, string>;
  focusKeyByModule: Record<ContentModule, string>;
  scrollPositionByModuleMode: Record<string, number>;
  quality: MotionQuality;
  muted: boolean;
  detailId: string | null;
  reviewOpen: boolean;
}

export interface TemplateViewProps {
  mode: ContentMode;
  module: ContentModule;
  items: ConceptContentItem[];
  selectedId: string;
  quality: MotionQuality;
  reducedMotion: boolean;
  onSelect: (id: string) => void;
  onOpen: (id: string) => void;
  onBack: () => void;
}

export interface TemplateRendererContract {
  id: ConceptTemplate;
  number: string;
  label: string;
  component: unknown;
}

export interface MotionDriverContract {
  quality: MotionQuality;
  reducedMotion: boolean;
  enter(root: HTMLElement): () => void;
  transition(root: HTMLElement, from: ContentMode, to: ContentMode): Promise<void>;
}

export interface MediaStageContract {
  mount(canvas: HTMLCanvasElement, assets: ConceptMediaAsset[]): Promise<void>;
  setActive(index: number, velocity?: number): void;
  resize(width: number, height: number, dpr: number): void;
  dispose(): void;
}

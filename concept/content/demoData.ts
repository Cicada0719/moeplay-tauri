import type {
  ConceptContentItem,
  ConceptMediaAsset,
  ConceptTemplate,
  ContentModule,
  MediaShotType,
} from "../contracts";
import manifest from "./media-manifest.json";

type ManifestAsset = Omit<ConceptMediaAsset, "mediaType"> & {
  title: string;
  downloadedFrom: string;
  mime: "image/jpeg" | "image/png";
  width: number;
  height: number;
  shotMapping: Record<ConceptTemplate, MediaShotType | "strip">;
  byteSize: number;
  sha256: string;
};

const assets = (manifest.assets as ManifestAsset[]).map<ConceptMediaAsset>((asset) => ({
  id: asset.id,
  contentId: asset.contentId,
  src: asset.src,
  mediaType: "image",
  ratio: asset.ratio,
  shotType: asset.shotType,
  tone: asset.tone,
  dominantColor: asset.dominantColor,
  focalPoint: asset.focalPoint,
  templateUsage: asset.templateUsage,
  sourceUrl: asset.sourceUrl,
}));

function mediaFor(contentId: string): ConceptMediaAsset[] {
  return assets.filter((asset) => asset.contentId === contentId);
}

export const DEMO_CONTENT: readonly ConceptContentItem[] = [
  {
    id: "wuwa-aemeath",
    module: "games",
    title: "爱弥斯 / Aemeath",
    subtitle: "毕业展之后，星光仍在回响",
    description: "以爱弥斯为主视觉的鸣潮角色档案：冷蓝舞台、角色设定与近景展示组成一条可被三种模板重新剪辑的镜头链。",
    status: "主展示 · 已归档",
    progress: 82,
    progressLabel: "共鸣链 / 82%",
    meta: ["鸣潮", "角色特写", "冷蓝", "主角"],
    media: mediaFor("wuwa-aemeath"),
  },
  {
    id: "wuwa-ensemble",
    module: "games",
    title: "鸣潮 · 共鸣者群像",
    subtitle: "守岸人、今汐、长离、椿与卡提希娅",
    description: "宽银幕场景与竖幅角色卡交错的群像章节，用于验证横竖素材在 cinematic、editorial 与 kinetic 中的差异化构图。",
    status: "群像专题 · 进行中",
    progress: 64,
    progressLabel: "角色档案 / 5 of 8",
    meta: ["鸣潮", "群像", "共鸣者", "多镜头"],
    media: mediaFor("wuwa-ensemble"),
  },
  {
    id: "anime-frieren",
    module: "anime",
    title: "葬送的芙莉莲",
    subtitle: "漫长旅途中的安静停顿",
    description: "以高亮留白和人物竖幅承接编辑画报式排版，为高速动态流提供一处低速、克制的呼吸段落。",
    status: "动画 · 想看",
    progress: 18,
    progressLabel: "旅程记录 / 18%",
    meta: ["动画", "芙莉莲", "静谧", "长旅"],
    media: mediaFor("anime-frieren"),
  },
  {
    id: "anime-chainsaw-man",
    module: "anime",
    title: "电锯人",
    subtitle: "噪点、冲突与绿色爆裂",
    description: "高反差关键视觉作为 kinetic 模板的能量峰值，也可在 cinematic 中作为章节突变镜头。",
    status: "动画 · 已看",
    progress: 100,
    progressLabel: "第一季 / 完成",
    meta: ["动画", "电锯人", "高能", "冲突"],
    media: mediaFor("anime-chainsaw-man"),
  },
  {
    id: "comics-chobits",
    module: "comics",
    title: "Chobits / 人形电脑天使心",
    subtitle: "千禧年蓝色封面档案",
    description: "20 周年版封面承担漫画模块的纸张感入口，在 editorial 中映射为 manga-page，在 kinetic 中映射为快速封面卡。",
    status: "漫画 · 收藏",
    progress: 46,
    progressLabel: "卷册 / 4 of 8",
    meta: ["漫画", "Chobits", "CLAMP", "千禧年"],
    media: mediaFor("comics-chobits"),
  },
] as const;

export const DEMO_CONTENT_BY_MODULE: Readonly<Record<ContentModule, readonly ConceptContentItem[]>> = {
  games: DEMO_CONTENT.filter((item) => item.module === "games"),
  anime: DEMO_CONTENT.filter((item) => item.module === "anime"),
  comics: DEMO_CONTENT.filter((item) => item.module === "comics"),
};

export const DEFAULT_SELECTED_ID_BY_MODULE: Readonly<Record<ContentModule, string>> = {
  games: "wuwa-aemeath",
  anime: "anime-frieren",
  comics: "comics-chobits",
};

export function getDemoContent(id: string): ConceptContentItem | undefined {
  return DEMO_CONTENT.find((item) => item.id === id);
}

export function getTemplateMedia(
  contentId: string,
  template: ConceptTemplate,
): readonly ConceptMediaAsset[] {
  return mediaFor(contentId).filter((asset) => asset.templateUsage.includes(template));
}



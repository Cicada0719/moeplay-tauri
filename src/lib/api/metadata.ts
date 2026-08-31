// api 域模块：metadata（由 index.ts 拆分而来，行为不变；index.ts 统一重导出）
import { invokeCmd } from "./core";
import type { Game, ScrapeResult, Settings, NsfwDisplayMode, NsfwDecision, ChineseMeta, ScrapeMarker } from "./types";

export async function getNsfwDecision(
  gameId: string,
  mode?: NsfwDisplayMode
): Promise<NsfwDecision> {
  return invokeCmd("get_nsfw_decision", { gameId, mode });
}


export async function classifyNsfwGame(
  game: Game,
  mode: NsfwDisplayMode = "blur"
): Promise<NsfwDecision> {
  return invokeCmd("classify_nsfw_game", { game, mode });
}


export async function getGamesNsfwFiltered(mode?: NsfwDisplayMode): Promise<Game[]> {
  return invokeCmd("get_games_nsfw_filtered", { mode });
}


export async function updateNsfwDisplayMode(mode: NsfwDisplayMode): Promise<Settings> {
  return invokeCmd("update_nsfw_display_mode", { mode });
}


export async function translateScrapeMetadata(
  result: ScrapeResult,
  targetLanguage: string | null = null
): Promise<ChineseMeta> {
  return invokeCmd("translate_scrape_metadata", { result, targetLanguage });
}


export async function translateText(
  text: string,
  targetLanguage: string | null = null
): Promise<string> {
  return invokeCmd("translate_text", { text, targetLanguage });
}


export async function parseChineseMetadata(text: string): Promise<ChineseMeta> {
  return invokeCmd("parse_chinese_metadata", { text });
}


export async function embedChineseMetadata(
  text: string | null,
  meta: ChineseMeta
): Promise<string> {
  return invokeCmd("embed_chinese_metadata", { text, meta });
}


export async function stripMetadataMarkers(text: string): Promise<string> {
  return invokeCmd("strip_metadata_markers", { text });
}


export async function parseScrapeMarker(text: string): Promise<ScrapeMarker> {
  return invokeCmd("parse_scrape_marker", { text });
}


export async function embedScrapeMarker(
  text: string | null,
  source: string | null,
  metadataHash: string | null,
  coverImage: boolean,
  backgroundImage: boolean
): Promise<string> {
  return invokeCmd("embed_scrape_marker", {
    text,
    source,
    metadataHash,
    coverImage,
    backgroundImage,
  });
}

// ===== 设置 =====


// api 域模块：scraper（由 index.ts 拆分而来，行为不变；index.ts 统一重导出）
import { invokeCmd } from "./core";
import type { Game, ScrapeResult, ScrapeResponse } from "./types";

export interface ScrapeSourceOptions {
  dlsite?: boolean;
  getchu?: boolean;
  touchgal?: boolean;
  erogamescape?: boolean;
  ymgal?: boolean;
  kungal?: boolean;
  steam?: boolean;
  pcgw?: boolean;
}


export async function scrapeGames(
  query: string,
  vndb: boolean,
  bangumi: boolean,
  sources: ScrapeSourceOptions = {}
): Promise<ScrapeResponse> {
  return invokeCmd("scrape_games", { query, vndb, bangumi, ...sources });
}


export async function scrapeGame(
  query: string,
  vndb: boolean,
  bangumi: boolean,
  sources: ScrapeSourceOptions = {}
): Promise<ScrapeResponse> {
  return invokeCmd("scrape_game", { query, vndb, bangumi, ...sources });
}


export async function scrapeKungalDetail(gameId: string): Promise<ScrapeResult> {
  return invokeCmd("scrape_kungal_detail", { gameId });
}


export async function scrapeSteamApp(appId: string): Promise<ScrapeResult> {
  return invokeCmd("scrape_steam_app", { appId });
}


export async function scrapePcgwPage(title: string): Promise<ScrapeResult> {
  return invokeCmd("scrape_pcgw_page", { title });
}

/** 在系统默认浏览器中打开 URL */

export async function openUrl(url: string): Promise<void> {
  return invokeCmd("open_url", { url });
}

/** 在系统文件管理器中打开路径（文件夹/文件） */

export async function openPath(path: string): Promise<void> {
  return invokeCmd("open_path", { path });
}

/** 搜索 Galgame 下载资源（TouchGAL/Kungal） */

export function buildSourceUrl(r: ScrapeResult): string | null {
  switch (r.source) {
    case "vndb":          return `https://vndb.org/${r.source_id}`;
    case "bangumi":       return `https://bgm.tv/subject/${r.source_id}`;
    case "steam":         return `https://store.steampowered.com/app/${r.source_id}`;
    case "dlsite":        return r.detail?.homepage ?? (r.source_id ? `https://www.dlsite.com/maniax/work/=/product_id/${r.source_id}.html` : null);
    case "erogamescape":  return `https://erogamescape.dyndns.org/~ap2/ero/toukei_kaiseki/game.php?game=${r.source_id}`;
    case "pcgw":          return `https://www.pcgamingwiki.com/wiki/${encodeURIComponent(r.title)}`;
    case "kungal":         return r.detail?.homepage ?? null;
    case "ymgal":          return r.detail?.homepage ?? null;
    case "touchgal":       return r.detail?.homepage ?? null;
    case "ai":             return null;
    default:               return r.detail?.homepage ?? null;
  }
}


export async function applyScrapeResult(
  gameId: string,
  result: ScrapeResult
): Promise<Game> {
  return invokeCmd("apply_scrape_result", { gameId, result });
}

/** 取某条搜索结果的全量详情（截图/开发商/发行商/流派/别名/发行日期等）。
 *  搜索只回浅层结果，落库前先用它把富字段补全。 */

export async function fetchFullDetail(
  source: string,
  sourceId: string
): Promise<ScrapeResult> {
  return invokeCmd("fetch_full_detail", { source, sourceId });
}

// ===== 存档（文件系统扫描） =====


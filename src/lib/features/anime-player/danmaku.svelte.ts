// 弹幕（DanDanPlay）独立存储（从 anime.svelte.ts 拆出）。
import { invokeCmd } from "../../api/core";

export interface DanmakuComment {
  time: number;
  mode: number; // 1=scroll, 4=bottom, 5=top
  color: number;
  text: string;
}

export interface DanmakuEpisode {
  episode_id: number;
  episode_title: string;
}

export interface DanmakuAnime {
  anime_id: number;
  anime_title: string;
  episodes: DanmakuEpisode[];
}

function loadJson<T>(key: string, fallback: T): T {
  try {
    const raw = localStorage.getItem(key);
    return raw ? (JSON.parse(raw) as T) : fallback;
  } catch {
    return fallback;
  }
}

function saveJson(key: string, data: unknown) {
  localStorage.setItem(key, JSON.stringify(data));
}

let _danmakuEnabled = $state(loadJson<boolean>("danmaku-enabled", true));
let _danmakuOpacity = $state(loadJson<number>("danmaku-opacity", 1));
let _danmakuSpeed = $state(loadJson<number>("danmaku-speed", 1));
let _danmakuFontSize = $state(loadJson<number>("danmaku-font-size", 24));
let _danmakuArea = $state(loadJson<number>("danmaku-area", 1));
let _danmakuBlockScroll = $state(loadJson<boolean>("danmaku-block-scroll", false));
let _danmakuBlockTop = $state(loadJson<boolean>("danmaku-block-top", false));
let _danmakuBlockBottom = $state(loadJson<boolean>("danmaku-block-bottom", false));
let _danmakuBlockWords = $state<string[]>(loadJson("danmaku-block-words", []));
let _danmakuComments = $state<DanmakuComment[]>([]);
let _danmakuLoading = $state(false);
let _danmakuAnimeId = $state(0);
let _danmakuEpisodeId = $state(0);

export const danmakuStore = {
  get enabled() { return _danmakuEnabled; },
  set enabled(v: boolean) { _danmakuEnabled = v; saveJson("danmaku-enabled", v); },
  get comments() { return _danmakuComments; },
  get loading() { return _danmakuLoading; },
  get animeId() { return _danmakuAnimeId; },
  get episodeId() { return _danmakuEpisodeId; },
  get opacity() { return _danmakuOpacity; },
  set opacity(v: number) { _danmakuOpacity = v; saveJson("danmaku-opacity", v); },
  get speed() { return _danmakuSpeed; },
  set speed(v: number) { _danmakuSpeed = v; saveJson("danmaku-speed", v); },
  get fontSize() { return _danmakuFontSize; },
  set fontSize(v: number) { _danmakuFontSize = v; saveJson("danmaku-font-size", v); },
  get area() { return _danmakuArea; },
  set area(v: number) { _danmakuArea = v; saveJson("danmaku-area", v); },
  get blockScroll() { return _danmakuBlockScroll; },
  set blockScroll(v: boolean) { _danmakuBlockScroll = v; saveJson("danmaku-block-scroll", v); },
  get blockTop() { return _danmakuBlockTop; },
  set blockTop(v: boolean) { _danmakuBlockTop = v; saveJson("danmaku-block-top", v); },
  get blockBottom() { return _danmakuBlockBottom; },
  set blockBottom(v: boolean) { _danmakuBlockBottom = v; saveJson("danmaku-block-bottom", v); },
  get blockWords() { return _danmakuBlockWords; },
  set blockWords(v: string[]) { _danmakuBlockWords = v; saveJson("danmaku-block-words", v); },

  async searchForAnime(animeName: string, episodeIdx?: number) {
    if (!animeName.trim()) return;
    _danmakuLoading = true;
    _danmakuComments = [];
    _danmakuAnimeId = 0;
    _danmakuEpisodeId = 0;
    try {
      const animes = await invokeCmd<DanmakuAnime[]>("anime_danmaku_search", { keyword: animeName });
      if (animes.length === 0) {
        _danmakuLoading = false;
        return;
      }
      const best = animes[0];
      _danmakuAnimeId = best.anime_id;
      if (best.episodes.length > 0) {
        const epNum = episodeIdx !== undefined ? episodeIdx + 1 : 1;
        const matchedEp = best.episodes.find((ep) => {
          const match = ep.episode_title.match(/(\d+)/);
          return match ? parseInt(match[1]) === epNum : false;
        }) || best.episodes[Math.min(episodeIdx ?? 0, best.episodes.length - 1)];
        if (matchedEp) {
          _danmakuEpisodeId = matchedEp.episode_id;
          await this.load(matchedEp.episode_id);
        }
      }
    } catch (e) {
      console.warn("弹幕搜索失败:", e);
    } finally {
      _danmakuLoading = false;
    }
  },

  async load(episodeId: number) {
    _danmakuLoading = true;
    try {
      _danmakuComments = await invokeCmd<DanmakuComment[]>("anime_danmaku_get_comments", { episodeId });
      _danmakuEpisodeId = episodeId;
    } catch (e) {
      console.warn("弹幕加载失败:", e);
      _danmakuComments = [];
    } finally {
      _danmakuLoading = false;
    }
  },
};

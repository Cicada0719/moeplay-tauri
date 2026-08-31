// 图片搜番（trace.moe）独立存储（从 anime.svelte.ts 拆出）。
import { invokeCmd } from "../../api/core";

export interface TraceMoeResult {
  anilist_id: number;
  filename: string;
  episode: string;
  from: number;
  to: number;
  similarity: number;
  video: string;
  image: string;
  title_native: string;
  title_chinese: string;
  title_english: string;
}

let _results = $state<TraceMoeResult[]>([]);
let _loading = $state(false);
let _error = $state<string | null>(null);

export const imageSearchStore = {
  get results() { return _results; },
  get loading() { return _loading; },
  get error() { return _error; },
  async search(imageUrl: string) {
    if (!imageUrl.trim()) return;
    _loading = true;
    _error = null;
    _results = [];
    try {
      _results = await invokeCmd<TraceMoeResult[]>("anime_image_search", { imageUrl });
    } catch (e) {
      _error = String(e);
      _results = [];
    } finally {
      _loading = false;
    }
  },
  clear() {
    _results = [];
    _error = null;
  },
};

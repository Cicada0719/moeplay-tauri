// 章节评论独立存储（从 anime.svelte.ts 拆出，降低主 store 复杂度的第一步）。
import { invokeCmd } from "../../api/core";

export interface BangumiEpisodeComment {
  user: string;
  avatar: string;
  comment: string;
  date: string;
}

let _comments = $state<BangumiEpisodeComment[]>([]);
let _loading = $state(false);

export const episodeCommentsStore = {
  get comments() { return _comments; },
  get loading() { return _loading; },
  async load(episodeId: number) {
    _loading = true;
    _comments = [];
    try {
      _comments = await invokeCmd<BangumiEpisodeComment[]>("anime_bangumi_episode_comments", { episodeId });
    } catch (e) {
      console.warn("章节评论加载失败:", e);
      _comments = [];
    } finally {
      _loading = false;
    }
  },
};

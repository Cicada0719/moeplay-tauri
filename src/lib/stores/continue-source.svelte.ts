// 继续观看/阅读数据源注册表：把 anime/comic store 的历史以轻量方式暴露给主包，
// 避免主入口静态加载整个 store（store 位于懒加载 chunk 中）。
// 两个 store 加载后会自动把历史同步进来（见各自文件末尾的 $effect.root）。
import type { AnimeHistory } from "./anime.svelte";
import type { ReadRecord } from "./comic.svelte";

let _animeHistory = $state<AnimeHistory[]>([]);
let _comicHistory = $state<ReadRecord[]>([]);

export const continueSource = {
  get animeHistory() { return _animeHistory; },
  get comicHistory() { return _comicHistory; },
  setAnimeHistory(items: AnimeHistory[]) { _animeHistory = items; },
  setComicHistory(items: ReadRecord[]) { _comicHistory = items; },
};

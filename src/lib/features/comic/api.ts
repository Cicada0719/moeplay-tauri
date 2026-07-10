import { invokeCmd } from "../../api/core";
import type {
  ComicChapter,
  ComicProviderApi,
  ComicProviderConfigureRequest,
  ComicProviderDescriptor,
  ComicProviderProbe,
  ComicResolvedTarget,
  ComicSearchRequest,
  ComicSeries,
  ComicSeriesDetail,
} from "./types";

/** All eight commands resolve through the same managed Rust registry. */
export const comicProviderApi: ComicProviderApi = {
  configure: (request) => invokeCmd<ComicProviderDescriptor>("comic_provider_configure", { request }),
  list: () => invokeCmd<ComicProviderDescriptor[]>("comic_provider_list"),
  remove: (providerId) => invokeCmd<boolean>("comic_provider_remove", { providerId }),
  probe: (providerId) => invokeCmd<ComicProviderProbe>("comic_provider_probe", { providerId }),
  search: (providerId, request) => invokeCmd<ComicSeries[]>("comic_provider_search", { providerId, request: { page: 1, pageSize: 50, ...request } }),
  detail: (providerId, seriesId) => invokeCmd<ComicSeriesDetail>("comic_provider_detail", { providerId, seriesId }),
  chapters: (providerId, seriesId) => invokeCmd<ComicChapter[]>("comic_provider_chapters", { providerId, seriesId }),
  resolve: (providerId, seriesId, chapterId) => invokeCmd<ComicResolvedTarget>("comic_provider_resolve", { providerId, request: { seriesId, chapterId } }),
};

export type { ComicProviderConfigureRequest, ComicSearchRequest };


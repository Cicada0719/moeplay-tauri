import type { PlayerQuality } from "../stores/player";

/**
 * switchQuality 的媒体重载判定（spec §3.3 回归契约）。
 *
 * 本地超清化画质档位（off / balanced / quality）是 enhancement 管线状态，与视频源无关：
 * - 纯增强模式切换（targetSrc 与当前已加载源 loadedSrc 相同）只更新 enhancement 管线状态，
 *   绝不重载媒体，避免同源 m3u8 全量重缓冲；
 * - 仅当目标源地址（animeStore.playerVideoSrc）相对当前已加载源确实变化时才返回 true，
 *   由调用方复用同一 <video> 元素替换 source 并恢复进度（spec §3.3）。
 */
export interface QualitySwitchReloadContext {
  /** 当前 video 元素（不存在时无法重载） */
  el: HTMLVideoElement | null;
  /** 目标源地址（animeStore.playerVideoSrc） */
  targetSrc: string;
  /** 当前实际加载到媒体元素上的源地址 */
  loadedSrc: string;
  /** 目标画质档位 */
  quality: PlayerQuality;
  /** 当前画质档位 */
  currentQuality: PlayerQuality;
}

export function shouldReloadMedia(ctx: QualitySwitchReloadContext): boolean {
  return Boolean(
    ctx.el &&
      ctx.targetSrc &&
      ctx.quality !== ctx.currentQuality &&
      ctx.targetSrc !== ctx.loadedSrc,
  );
}

// 图片宽高探测辅助（FR-11 跨页大图识别的前置步骤）
//
// 用浏览器 `Image` 加载图片并读取 naturalWidth/naturalHeight，结果缓存进模块级
// Map 供 `buildScreens` 前的 PageMeta 装配使用。加载失败按 {0,0} 记录，调用方
// 按普通页处理。

export interface ImageSize {
  width: number;
  height: number;
}

const sizeCache = new Map<string, ImageSize>();

/** 探测图片宽高。命中缓存直接返回；失败返回 {0,0}（不抛出）。 */
export function probeImageSize(url: string): Promise<ImageSize> {
  const cached = sizeCache.get(url);
  if (cached) return Promise.resolve(cached);

  return new Promise<ImageSize>((resolve) => {
    let settled = false;
    const done = (size: ImageSize) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      sizeCache.set(url, size);
      resolve(size);
    };

    const img = new Image();
    img.onload = () => done({ width: img.naturalWidth, height: img.naturalHeight });
    img.onerror = () => done({ width: 0, height: 0 });
    img.src = url;

    // 极慢 / 挂起的图片不阻塞首屏：兜底 8s 超时按普通页处理。
    const timer = setTimeout(() => done({ width: 0, height: 0 }), 8000);
  });
}

/** 同步读取缓存中的尺寸（无缓存返回 undefined）。 */
export function getCachedImageSize(url: string): ImageSize | undefined {
  return sizeCache.get(url);
}

/** 清空尺寸缓存（测试隔离用）。 */
export function clearImageSizeCache(): void {
  sizeCache.clear();
}

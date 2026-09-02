export interface PlayerTransportSupport {
  isM3u8: boolean;
  nativeHls: boolean;
  hlsSupported: boolean;
  isAndroid: boolean;
}

/**
 * Pick the first transport for an extracted stream.
 *
 * Android WebView commonly reports native HLS as "maybe" even when the
 * proxied playlist cannot complete its first segment request. hls.js can
 * attach the proxy's CORS/Referer-aware MediaSource path, so Android should
 * prefer it and retain native video as the existing second attempt.
 */
export function shouldPreferHls({
  isM3u8,
  nativeHls,
  hlsSupported,
  isAndroid,
}: PlayerTransportSupport): boolean {
  return isM3u8 && hlsSupported && (isAndroid || !nativeHls);
}

/** Observe decoded video frames: audio time and loaded metadata are not a picture. */
export function watchVideoProgress(
  video: HTMLVideoElement,
  onFailure: (reason: string) => void,
  timeoutMs = 15_000,
) {
  let stopped = false;
  let frameHandle: number | undefined;
  let lastProgress = Date.now();
  let hasFrame = false;
  let frameCount = 0;
  let previousTime = video.currentTime;
  const supportsFrames = typeof video.requestVideoFrameCallback === 'function';
  const progress = () => { hasFrame = true; lastProgress = Date.now(); };
  const schedule = () => {
    if (stopped || !supportsFrames) return;
    frameHandle = video.requestVideoFrameCallback(() => {
      if (stopped) return;
      progress();
      schedule();
    });
  };
  const reset = () => {
    hasFrame = false;
    frameCount = 0;
    previousTime = video.currentTime;
    lastProgress = Date.now();
  };
  const seek = () => { lastProgress = Date.now(); };
  const timer = setInterval(() => {
    if (stopped) return;
    if (document.visibilityState === 'hidden' || (hasFrame && (video.paused || video.ended))) {
      lastProgress = Date.now();
      return;
    }
    if (!supportsFrames && video.videoWidth > 0 && video.videoHeight > 0 && video.readyState >= 2) {
      const count = video.getVideoPlaybackQuality?.().totalVideoFrames;
      if (typeof count === 'number' ? count > frameCount : video.currentTime !== previousTime) progress();
      frameCount = count ?? frameCount;
      previousTime = video.currentTime;
    }
    if (Date.now() - lastProgress >= timeoutMs) {
      const reason = hasFrame ? '视频画面长时间未更新，请重试或切换来源' : '未能解码视频画面（可能只有音轨），请切换来源或使用外部播放器';
      dispose();
      onFailure(reason);
    }
  }, 500);
  function dispose() {
    if (stopped) return;
    stopped = true;
    clearInterval(timer);
    if (frameHandle !== undefined) video.cancelVideoFrameCallback?.(frameHandle);
    video.removeEventListener('seeking', seek);
    video.removeEventListener('seeked', seek);
  }
  video.addEventListener('seeking', seek);
  video.addEventListener('seeked', seek);
  schedule();
  return { dispose, reset, hasFrame: () => hasFrame };
}

export function releaseVideo(video: HTMLVideoElement): void {
  video.pause();
  video.removeAttribute('src');
  video.load();
}

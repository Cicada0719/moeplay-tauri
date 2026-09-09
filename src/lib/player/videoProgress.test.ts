import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { releaseVideo, watchVideoProgress } from './videoProgress';

describe('decoded picture progress', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    Object.defineProperty(document, 'visibilityState', { configurable: true, value: 'visible' });
  });
  afterEach(() => { vi.useRealTimers(); vi.restoreAllMocks(); });
  function media() {
    const video = document.createElement('video');
    Object.defineProperties(video, {
      readyState: { configurable: true, value: 4 },
      videoWidth: { configurable: true, value: 0 },
      videoHeight: { configurable: true, value: 0 },
      paused: { configurable: true, value: false },
    });
    return video;
  }
  it('does not accept an advancing audio clock as decoded video', () => {
    const video = media();
    const fail = vi.fn();
    watchVideoProgress(video, fail, 2000);
    video.currentTime = 750;
    vi.advanceTimersByTime(2000);
    expect(fail).toHaveBeenCalledOnce();
    expect(fail.mock.calls[0][0]).toContain('只有音轨');
    vi.advanceTimersByTime(2000);
    expect(fail).toHaveBeenCalledOnce();
  });
  it('observes real frames and detects later frozen pictures', () => {
    const video = media();
    let frame: VideoFrameRequestCallback | undefined;
    video.requestVideoFrameCallback = vi.fn(callback => { frame = callback; return 1; });
    video.cancelVideoFrameCallback = vi.fn();
    const fail = vi.fn();
    const watch = watchVideoProgress(video, fail, 2000);
    vi.advanceTimersByTime(1500);
    frame!(0, {} as VideoFrameCallbackMetadata);
    expect(watch.hasFrame()).toBe(true);
    vi.advanceTimersByTime(1500);
    expect(fail).not.toHaveBeenCalled();
    vi.advanceTimersByTime(500);
    expect(fail.mock.calls[0][0]).toContain('长时间未更新');
    expect(video.cancelVideoFrameCallback).toHaveBeenCalled();
  });
  it('pauses the timeout while hidden and cancels it on cleanup', () => {
    const video = media();
    const fail = vi.fn();
    const watch = watchVideoProgress(video, fail, 2000);
    Object.defineProperty(document, 'visibilityState', { configurable: true, value: 'hidden' });
    vi.advanceTimersByTime(5000);
    expect(fail).not.toHaveBeenCalled();
    Object.defineProperty(document, 'visibilityState', { configurable: true, value: 'visible' });
    watch.dispose();
    vi.advanceTimersByTime(5000);
    expect(fail).not.toHaveBeenCalled();
  });
  it('releases native audio and source when leaving a player', () => {
    const video = media();
    video.setAttribute('src', 'https://media.test/a.mp4');
    video.pause = vi.fn();
    video.load = vi.fn();
    releaseVideo(video);
    expect(video.pause).toHaveBeenCalledOnce();
    expect(video.hasAttribute('src')).toBe(false);
    expect(video.load).toHaveBeenCalledOnce();
  });
});

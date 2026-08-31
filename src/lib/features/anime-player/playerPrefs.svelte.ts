// 播放器偏好（localStorage 持久化）独立存储（从 anime.svelte.ts 拆出）。
import { normalizeVideoEnhancementMode, type VideoEnhancementMode } from "./localVideoEnhancement";

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

let _autoNext = $state(loadJson<boolean>("player-auto-next", true));
let _playbackRate = $state(loadJson<number>("player-playback-rate", 1));
let _longPressRate = $state(loadJson<number>("player-long-press-rate", 3));
let _skipOpening = $state(loadJson<number>("player-skip-opening", 0));
let _skipEnding = $state(loadJson<number>("player-skip-ending", 0));
let _autoWebFallback = $state(loadJson<boolean>("player-auto-web-fallback", true));
let _videoEnhancementMode = $state<VideoEnhancementMode>(normalizeVideoEnhancementMode(loadJson<unknown>("player-video-enhancement", "off")));

export const playerPrefs = {
  get autoNext() { return _autoNext; },
  set autoNext(v: boolean) { _autoNext = v; saveJson("player-auto-next", v); },
  get playbackRate() { return _playbackRate; },
  set playbackRate(v: number) { _playbackRate = v; saveJson("player-playback-rate", v); },
  get longPressRate() { return _longPressRate; },
  set longPressRate(v: number) { _longPressRate = v; saveJson("player-long-press-rate", v); },
  get skipOpening() { return _skipOpening; },
  set skipOpening(v: number) { _skipOpening = v; saveJson("player-skip-opening", v); },
  get skipEnding() { return _skipEnding; },
  set skipEnding(v: number) { _skipEnding = v; saveJson("player-skip-ending", v); },
  get autoWebFallback() { return _autoWebFallback; },
  set autoWebFallback(v: boolean) { _autoWebFallback = v; saveJson("player-auto-web-fallback", v); },
  get videoEnhancementMode() { return _videoEnhancementMode; },
  set videoEnhancementMode(v: VideoEnhancementMode) {
    _videoEnhancementMode = normalizeVideoEnhancementMode(v);
    saveJson("player-video-enhancement", _videoEnhancementMode);
  },
};

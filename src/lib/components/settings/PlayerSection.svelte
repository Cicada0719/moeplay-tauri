<script lang="ts">
  import { animeStore } from "../../stores/anime.svelte";
  import { i18n } from "../../stores/i18n.svelte";
  import Card from "../ui/Card.svelte";
  import Switch from "../ui/Switch.svelte";
  import Input from "../ui/Input.svelte";
  import SegmentControl from "../ui/SegmentControl.svelte";
  import Icon from "../Icon.svelte";
  import "./settings-shared.css";
</script>

<span class="section-anchor" id="settings-player" aria-hidden="true"></span>
<Card class="s-section" padding="lg" ariaLabel="settings-player">
  <div class="s-head">
    <h2 class="s-title"><Icon name="film" size={17} className="s-title-ic" /> {i18n.t("settings.section_player")}<span class="s-title-sub">再生 / PLAYER</span></h2>
  </div>

  <div class="src-item">
    <div class="src-info">
      <span class="src-name">自动连播</span>
      <span class="src-desc">一集播完后自动播放下一集</span>
    </div>
    <Switch checked={animeStore.autoNext} onchange={() => animeStore.autoNext = !animeStore.autoNext} />
  </div>

  <div class="src-item">
    <div class="src-info">
      <span class="src-name">默认开启弹幕</span>
      <span class="src-desc">进入播放器时自动加载弹幕</span>
    </div>
    <Switch checked={animeStore.danmakuEnabled} onchange={() => animeStore.danmakuEnabled = !animeStore.danmakuEnabled} />
  </div>

  <div class="s-divider"></div>

  <div class="s-row">
    <div class="s-info">
      <span class="s-label">跳过片头（秒）</span>
      <span class="s-desc">每集开始自动跳到第 N 秒，0 表示不跳</span>
    </div>
    <Input
      class="num-input"
      type="number"
      min="0"
      max="300"
      step="1"
      value={String(animeStore.skipOpening)}
      oninput={(e) => animeStore.skipOpening = Math.max(0, parseInt((e.target as HTMLInputElement).value) || 0)}
    />
  </div>

  <div class="s-row">
    <div class="s-info">
      <span class="s-label">跳过片尾（秒）</span>
      <span class="s-desc">距结尾 N 秒时自动跳下一集，0 表示不跳</span>
    </div>
    <Input
      class="num-input"
      type="number"
      min="0"
      max="300"
      step="1"
      value={String(animeStore.skipEnding)}
      oninput={(e) => animeStore.skipEnding = Math.max(0, parseInt((e.target as HTMLInputElement).value) || 0)}
    />
  </div>

  <div class="s-divider"></div>

  <div class="s-row" style="flex-direction: column; align-items: flex-start; gap: 8px;">
    <div class="s-info">
      <span class="s-label">默认倍速</span>
      <span class="s-desc">进入播放器时的初始倍速</span>
    </div>
    <SegmentControl
      options={[0.5, 0.75, 1.0, 1.25, 1.5, 2.0, 3.0].map(r => ({ value: String(r), label: `${r}x` }))}
      value={String(animeStore.playbackRate)}
      onChange={(v) => animeStore.playbackRate = parseFloat(v)}
      size="sm"
    />
  </div>

  <div class="s-row" style="flex-direction: column; align-items: flex-start; gap: 8px;">
    <div class="s-info">
      <span class="s-label">长按倍速</span>
      <span class="s-desc">长按画面时临时切换到的倍速</span>
    </div>
    <SegmentControl
      options={[1.5, 2.0, 3.0, 4.0].map(r => ({ value: String(r), label: `${r}x` }))}
      value={String(animeStore.longPressRate)}
      onChange={(v) => animeStore.longPressRate = parseFloat(v)}
      size="sm"
    />
  </div>
  <div class="s-divider"></div>

  <div class="s-row" style="flex-direction: column; align-items: flex-start; gap: 8px;">
    <div class="s-info">
      <span class="s-label">本地超清化</span>
      <span class="s-desc">使用 GPU 在本机放大并强化动画线条；均衡最高 1080p，质量优先最高 1440p。不会上传视频。</span>
    </div>
    <SegmentControl
      options={[
        { value: 'off', label: '关闭' },
        { value: 'balanced', label: '均衡' },
        { value: 'quality', label: '质量优先' },
      ]}
      value={animeStore.videoEnhancementMode}
      onChange={(v) => animeStore.videoEnhancementMode = v as 'off' | 'balanced' | 'quality'}
      size="sm"
    />
  </div>
</Card>

<script lang="ts">
  import { onMount } from "svelte";
  import Icon from "./Icon.svelte";
  import Skeleton from "./ui/Skeleton.svelte";
  import Button from "./ui/Button.svelte";
  import Card from "./ui/Card.svelte";
  import Tag from "./ui/Tag.svelte";
  import { uiStore } from "../stores/ui.svelte";
  import type { ScrapeResult } from "../api";
  import { openUrl, buildSourceUrl, searchGameDownloads, searchDownloadsDirect, downloadStart, type DownloadEntry, type DownloadSearchResult } from "../api";

  let { result, onClose }: { result: ScrapeResult; onClose: () => void } = $props();

  let dlResult = $state<DownloadSearchResult | null>(null);
  let dlLoading = $state(false);
  let dlError = $state("");
  let screenshotIdx = $state(0);

  const screenshots = $derived(result.detail?.screenshots ?? []);
  const sourceUrl = $derived(buildSourceUrl(result));

  const kindLabels: Record<string, string> = {
    magnet: "磁力链接",
    http: "直接下载",
    baidu_pan: "百度网盘",
    one_drive: "OneDrive",
    google_drive: "Google Drive",
    patch: "补丁",
    translation_patch: "汉化补丁",
    official_site: "官网",
    other: "其他",
  };

  const kindVariants: Record<string, "accent" | "muted" | "neutral"> = {
    magnet: "accent",
    http: "accent",
    baidu_pan: "muted",
    one_drive: "muted",
    google_drive: "muted",
    patch: "neutral",
    translation_patch: "neutral",
    official_site: "muted",
    other: "muted",
  };

  let detailExpanded = $state(true);

  onMount(() => {
    searchDownloads();
  });

  async function searchDownloads() {
    dlLoading = true;
    dlError = "";
    try {
      // 构建搜索候选名列表（优先级：原名 > 英文/日文 > 中文标题）
      const candidates = buildSearchNames(result);

      // 双通路：先走 Kungal API，失败则走 TouchGAL 直搜
      if (result.source === "kungal" || result.source === "touchgal") {
        dlResult = await searchGameDownloads(result.title, result.source_id, undefined);
      }
      if (!dlResult?.entries?.length) {
        dlResult = await searchDownloadsDirect(candidates);
      }
    } catch (e) {
      try {
        dlResult = await searchDownloadsDirect(buildSearchNames(result));
      } catch (e2) {
        dlError = String(e2);
      }
    } finally {
      dlLoading = false;
    }
  }

  /** 从 ScrapeResult 中提取搜索候选名（优先级降序） */
  function buildSearchNames(r: ScrapeResult): string[] {
    const names: string[] = [];

    // 1. 原名（original_name）
    if (r.detail?.original_name) names.push(r.detail.original_name);

    // 2. aliases 中过滤出英文/日文（排除纯中文）
    if (r.detail?.aliases) {
      for (const a of r.detail.aliases) {
        if (!names.includes(a) && !isOnlyCJK(a)) names.push(a);
      }
    }

    // 3. aliases 中的中文名
    if (r.detail?.aliases) {
      for (const a of r.detail.aliases) {
        if (!names.includes(a)) names.push(a);
      }
    }

    // 4. 标题本身
    if (!names.includes(r.title)) names.push(r.title);

    // 5. 修剪超长名称（>80 字符的通常不是好搜索词）
    return names.filter(n => n.length > 0 && n.length < 80);
  }

  /** 判断字符串是否主要由 CJK 字符组成（中文/日文汉字/假名） */
  function isOnlyCJK(s: string): boolean {
    let cjk = 0;
    for (const ch of s) {
      const code = ch.charCodeAt(0);
      if ((code >= 0x4e00 && code <= 0x9fff) ||  // CJK Unified
          (code >= 0x3040 && code <= 0x30ff) ||  // Hiragana + Katakana
          (code >= 0x3400 && code <= 0x4dbf)) {  // CJK Extension A
        cjk++;
      }
    }
    // >50% 是 CJK 则认为主要靠 CJK 无法在 TouchGAL 搜到
    return cjk > 0 && cjk / s.length > 0.5;
  }

  async function startDownload(entry: DownloadEntry) {
    if (!entry.direct_download) {
      openUrl(entry.url);
      return;
    }
    try {
      await downloadStart(entry.url, entry.label + ".tmp", true, true);
    } catch (e) {
      console.error("Start download failed:", e);
      openUrl(entry.url);
    }
    // Navigate to download page
    uiStore.pendingDownloadUrl = entry.url;
    uiStore.pendingDownloadName = entry.label;
    uiStore.currentView = "downloads";
  }

  function handleEntryClick(entry: DownloadEntry) {
    if (entry.direct_download) {
      startDownload(entry);
    } else {
      // Non-HTTP: open in browser AND navigate to downloads page
      openUrl(entry.url);
      uiStore.currentView = "downloads";
    }
  }

  function openDownloadSource() {
    if (dlResult?.source_url) openUrl(dlResult.source_url);
  }

  function handleOverlayClick(e: MouseEvent) {
    if (e.target === e.currentTarget) onClose();
  }

  function handleOverlayKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      onClose();
    }
  }
</script>

<div class="overlay" onclick={handleOverlayClick} onkeydown={handleOverlayKeydown} role="dialog" tabindex="-1" aria-modal="true">
  <div class="detail-panel glass-card" role="document" tabindex="-1">

    <!-- Header -->
    <header class="detail-header">
      <Button variant="ghost" size="sm" onclick={onClose}>
        <Icon name="arrowLeft" size={18} /> 返回搜索
      </Button>
      {#if sourceUrl}
        <Button variant="secondary" size="sm" onclick={() => openUrl(sourceUrl)}>
          <Icon name="globe" size={14} /> {result.source.toUpperCase()}
        </Button>
      {/if}
    </header>

    <div class="detail-body">
      <!-- LEFT: game info -->
      <aside class="info-panel">
        {#if result.cover}
          <div class="cover-wrap">
            <img src={result.cover} alt={result.title} />
            {#if result.detail?.age_rating}
              <Tag variant="accent" size="sm" class="age-badge">{result.detail.age_rating}</Tag>
            {/if}
          </div>
        {:else}
          <div class="cover-placeholder">
            <Icon name="gamepad" size={48} />
          </div>
        {/if}

        <h1 class="title">{result.title}</h1>

        <div class="stats-row">
          {#if result.release_year}
            <span class="stat"><span class="stat-label">发行</span> {result.release_year}</span>
          {/if}
          {#if result.rating}
            <span class="stat"><span class="stat-label">评分</span> <Icon name="star" size={13} /> {result.rating.toFixed(1)}</span>
          {/if}
          {#if result.detail?.developer}
            <span class="stat"><span class="stat-label">开发商</span> {result.detail.developer}</span>
          {/if}
          {#if result.detail?.engine}
            <span class="stat"><span class="stat-label">引擎</span> {result.detail.engine}</span>
          {/if}
        </div>

        {#if result.tags.length}
          <div class="tags-wrap">
            {#each result.tags as tag}
              <Tag variant="neutral" size="sm">{tag}</Tag>
            {/each}
          </div>
        {/if}

        {#if result.description}
          <div class="desc-section" class:is-collapsed={!detailExpanded}>
            <h3>简介</h3>
            <p>{result.description}</p>
          </div>
        {:else}
          <div class="desc-section">
            <h3>简介</h3>
            <p class="muted">暂无简介数据</p>
          </div>
        {/if}

        <!-- Aliases -->
        {#if result.detail?.aliases?.length}
          <div class="aliases-section">
            <h3>别名</h3>
            <div class="aliases-list">
              {#each result.detail.aliases as alias}
                <span class="alias">{alias}</span>
              {/each}
            </div>
          </div>
        {/if}

        <!-- Genres -->
        {#if result.detail?.genres?.length}
          <div class="genres-section">
            <h3>类型</h3>
            <div class="tags-wrap">
              {#each result.detail.genres as genre}
                <Tag variant="accent" size="sm">{genre}</Tag>
              {/each}
            </div>
          </div>
        {/if}

        <!-- Action buttons -->
        <div class="actions-bar">
          {#if sourceUrl}
            <Button variant="primary" onclick={() => openUrl(sourceUrl!)}>
              <Icon name="globe" size={16} /> 打开 {result.source.toUpperCase()}
            </Button>
          {/if}
          <Button variant="ghost" onclick={() => detailExpanded = !detailExpanded}>
            <Icon name={detailExpanded ? "chevronDown" : "arrowLeft"} size={14} />
            {detailExpanded ? "收起" : "展开"}
          </Button>
        </div>
      </aside>

      <!-- RIGHT: resources + screenshots -->
      <main class="resource-panel">
        <!-- Screenshots gallery -->
        {#if screenshots.length > 0}
          <section class="section">
            <h3><Icon name="collection" size={16} /> 截图图集 ({screenshots.length})</h3>
            {#if screenshots.length > 1}
              <div class="screenshot-nav">
                {#each screenshots as _, i}
                  <button
                    class="nav-dot"
                    class:active={i === screenshotIdx}
                    onclick={() => screenshotIdx = i}
                    aria-label={`查看第 ${i + 1} 张截图`}
                    title={`第 ${i + 1} 张截图`}
                    type="button"
                  ></button>
                {/each}
              </div>
            {/if}
            <div class="screenshot-viewer">
              <img src={screenshots[screenshotIdx]} alt={`截图 ${screenshotIdx + 1}`} loading="lazy" />
              {#if screenshots.length > 1}
                <button
                  class="nav-arrow left"
                  onclick={() => screenshotIdx = (screenshotIdx - 1 + screenshots.length) % screenshots.length}
                  aria-label="上一张截图"
                  title="上一张截图"
                  type="button"
                >
                  <Icon name="arrowLeft" size={20} />
                </button>
                <button
                  class="nav-arrow right"
                  onclick={() => screenshotIdx = (screenshotIdx + 1) % screenshots.length}
                  aria-label="下一张截图"
                  title="下一张截图"
                  type="button"
                >
                  <Icon name="arrowLeft" size={20} />
                </button>
              {/if}
            </div>
          </section>
        {/if}

        <!-- Source links -->
        <section class="section">
          <h3><Icon name="globe" size={14} /> 数据源</h3>
          <div class="source-links">
            {#each [{ label: "VNDB", id: result.detail?.vndb_id, url: result.detail?.vndb_id ? `https://vndb.org/${result.detail?.vndb_id}` : null }, { label: "Bangumi", id: result.detail?.bangumi_id, url: result.detail?.bangumi_id ? `https://bgm.tv/subject/${result.detail?.bangumi_id}` : null }, { label: "Steam", id: result.source === "steam" ? result.source_id : null, url: result.source === "steam" ? `https://store.steampowered.com/app/${result.source_id}` : null }, { label: "DLsite", id: result.detail?.dl_site_id, url: null }] as link}
              {#if link.url}
                <Button variant="ghost" size="sm" onclick={() => openUrl(link.url!)}>
                  <Icon name="globe" size={14} /> {link.label}
                </Button>
              {/if}
            {/each}
            {#if sourceUrl}
              <Button variant="ghost" size="sm" onclick={() => openUrl(sourceUrl!)}>
                <Icon name="globe" size={14} /> 源站 ({result.source})
              </Button>
            {/if}
          </div>
        </section>

        <!-- Download resources -->
        <section class="section">
          <div class="section-header">
            <h3><Icon name="download" size={14} /> 下载资源</h3>
            {#if !dlResult && !dlLoading}
              <Button variant="ghost" size="sm" onclick={searchDownloads} disabled={dlLoading}>
                <Icon name="refresh" size={14} /> 搜索下载
              </Button>
            {/if}
          </div>

          {#if dlLoading}
            <Skeleton variant="card" count={3} />
            <p class="muted">正在从 TouchGAL / Kungal 搜索下载资源...</p>
          {:else if dlError}
            <p class="muted error"><Icon name="x" size={14} /> {dlError}</p>
            <Button variant="secondary" size="sm" onclick={searchDownloads}>重试</Button>
          {:else if dlResult?.entries?.length}
            {#if dlResult.source_url}
              <div class="source-note">
                数据来自 <button class="link" onclick={openDownloadSource} type="button">TouchGAL</button>
              </div>
            {/if}
            <div class="resource-list">
              {#each dlResult.entries as entry}
                <Card class="resource-card" hoverable onclick={() => handleEntryClick(entry)} padding="md">
                  <div class="res-header">
                    <Tag variant={kindVariants[entry.type] ?? "muted"} size="sm">{kindLabels[entry.type] ?? entry.type}</Tag>
                    {#if entry.size}
                      <span class="res-size text-mono">{entry.size}</span>
                    {/if}
                    <span class="res-action">
                      {#if entry.direct_download}
                        <Icon name="download" size={14} /> 下载
                      {:else}
                        <Icon name="globe" size={14} /> 打开
                      {/if}
                    </span>
                  </div>
                  <div class="res-label">{entry.label}</div>
                  {#if entry.note}
                    <div class="res-note">{entry.note}</div>
                  {/if}
                </Card>
              {/each}
            </div>
          {:else}
            <p class="muted">未找到下载资源。可能该游戏尚未收录，或 TouchGAL 不可达。</p>
            <Button variant="secondary" size="sm" onclick={searchDownloads}>重新搜索</Button>
          {/if}
        </section>
      </main>
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed; inset: 0; z-index: 100;
    background: rgba(0,0,0,0.55);
    display: flex; align-items: center; justify-content: center;
    animation: fadeIn 0.2s ease;
  }
  .detail-panel {
    width: 96vw; max-width: 1100px; height: 90vh;
    border-radius: var(--radius-xl); overflow: hidden;
    display: flex; flex-direction: column;
    animation: scaleIn 0.3s cubic-bezier(0.34, 1.56, 0.64, 1);
  }

  .detail-header {
    display: flex; align-items: center; justify-content: space-between;
    padding: 14px 20px; border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .detail-body {
    flex: 1; overflow-y: auto; display: grid;
    grid-template-columns: 1fr 1fr; gap: 0;
  }

  /* ===== LEFT PANEL ===== */
  .info-panel {
    padding: 24px; overflow-y: auto;
    border-right: 1px solid var(--border);
    display: flex; flex-direction: column; gap: 16px;
  }
  .cover-wrap { position: relative; border-radius: var(--radius-lg); overflow: hidden; }
  .cover-wrap img {
    width: 100%; max-height: 360px; object-fit: cover; display: block;
    aspect-ratio: 16/9;
  }
  .cover-placeholder {
    width: 100%; aspect-ratio: 16/9; border-radius: var(--radius-lg);
    background: var(--bg-hover); display: flex; align-items: center; justify-content: center;
    color: var(--text-muted);
  }
  :global(.age-badge) {
    position: absolute; top: 10px; right: 10px;
  }

  .title { font-size: 1.6rem; font-weight: 700; color: var(--text-primary); line-height: 1.2; }

  .stats-row { display: flex; flex-wrap: wrap; gap: 10px 18px; }
  .stat { font-family: var(--font-mono); font-size: 0.8rem; color: var(--text-primary); display: inline-flex; align-items: center; gap: 4px; }
  .stat-label { color: var(--text-muted); font-family: var(--font-ui); font-size: 0.7rem; text-transform: uppercase; letter-spacing: 0.5px; }

  .tags-wrap { display: flex; flex-wrap: wrap; gap: 6px; }

  .desc-section, .aliases-section, .genres-section { display: flex; flex-direction: column; gap: 8px; }
  .desc-section h3, .aliases-section h3, .genres-section h3 { font-size: 0.75rem; font-weight: 600; color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.5px; }
  .desc-section p { font-size: 0.85rem; color: var(--text-secondary); line-height: 1.7; }
  .desc-section.is-collapsed p {
    display: -webkit-box; -webkit-line-clamp: 4; line-clamp: 4; -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .muted { font-style: italic; opacity: 0.4; }

  .aliases-list { display: flex; flex-wrap: wrap; gap: 6px; }
  .alias { font-size: 0.78rem; color: var(--text-secondary); }
  .alias::after { content: " · "; color: var(--border); }
  .alias:last-child::after { content: ""; }

  .actions-bar { display: flex; gap: 8px; margin-top: auto; padding-top: 12px; border-top: 1px solid var(--border); }

  /* ===== RIGHT PANEL ===== */
  .resource-panel {
    padding: 24px; overflow-y: auto;
    display: flex; flex-direction: column; gap: 24px;
  }
  .section { display: flex; flex-direction: column; gap: 12px; }
  .section h3 { font-size: 0.85rem; font-weight: 600; color: var(--text-primary); display: inline-flex; align-items: center; gap: 6px; }

  /* Screenshots */
  .screenshot-viewer { position: relative; border-radius: var(--radius-lg); overflow: hidden; background: var(--bg-secondary); }
  .screenshot-viewer img { width: 100%; aspect-ratio: 16/9; object-fit: contain; display: block; }
  .screenshot-nav { display: flex; gap: 6px; }
  .nav-dot { width: 8px; height: 8px; border-radius: 50%; border: none; background: var(--bg-hover); cursor: pointer; transition: all 0.2s; }
  .nav-dot.active { background: var(--accent); }
  .nav-arrow {
    position: absolute; top: 50%; transform: translateY(-50%);
    width: 36px; height: 36px; border-radius: 50%; border: none;
    background: rgba(0,0,0,0.5); color: #fff; cursor: pointer;
    display: flex; align-items: center; justify-content: center;
    opacity: 0; transition: opacity 0.2s;
  }
  .screenshot-viewer:hover .nav-arrow { opacity: 1; }
  .nav-arrow.left { left: 8px; }
  .nav-arrow.right { right: 8px; transform: translateY(-50%) rotate(180deg); }

  /* Source links */
  .source-links { display: flex; flex-wrap: wrap; gap: 6px; }

  /* Download resources */
  .section-header { display: flex; align-items: center; justify-content: space-between; }
  .source-note { font-size: 0.75rem; color: var(--text-muted); }
  .source-note .link { background: none; border: none; color: var(--accent); cursor: pointer; text-decoration: underline; font-size: 0.75rem; padding: 0; }
  .error { color: var(--color-error) !important; }
  .resource-list { display: flex; flex-direction: column; gap: 8px; }
  :global(.resource-card) {
    width: 100%; text-align: left;
    cursor: pointer; text-decoration: none;
    font-family: var(--font-ui); color: var(--text-primary);
  }
  .res-header { display: flex; align-items: center; gap: 8px; }
  .res-size { font-size: 0.7rem; color: var(--text-muted); }
  .res-action { margin-left: auto; display: flex; align-items: center; gap: 4px; font-size: 0.75rem; color: var(--accent); }
  .res-label { font-size: 0.85rem; font-weight: 500; color: var(--text-primary); }
  .res-note { font-size: 0.75rem; color: var(--text-muted); line-height: 1.4; }

  @keyframes fadeIn { from { opacity: 0; } to { opacity: 1; } }
  @keyframes scaleIn { from { opacity: 0; transform: scale(0.96); } to { opacity: 1; transform: scale(1); } }

  @media (max-width: 720px) {
    .detail-body { grid-template-columns: 1fr; }
    .info-panel { border-right: none; border-bottom: 1px solid var(--border); }
  }
</style>

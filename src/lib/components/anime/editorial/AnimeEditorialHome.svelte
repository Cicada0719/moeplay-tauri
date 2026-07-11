<script lang="ts">
  import type { AnimeHistory, BangumiSubject } from "../../../stores/anime.svelte";

  interface Props {
    history?: readonly AnimeHistory[];
    seasonal?: readonly BangumiSubject[];
    trending?: readonly BangumiSubject[];
    topRated?: readonly BangumiSubject[];
    seasonalLoading?: boolean;
    trendingLoading?: boolean;
    topRatedLoading?: boolean;
    seasonalMore?: boolean;
    trendingMore?: boolean;
    topRatedMore?: boolean;
    getImage: (url: string) => string;
    onOpenSubject: (subject: BangumiSubject, trigger: HTMLElement) => void;
    onResumeHistory: (item: AnimeHistory, trigger: HTMLElement) => void;
    onMoreSeasonal?: () => void | Promise<void>;
    onMoreTrending?: () => void | Promise<void>;
    onMoreTopRated?: () => void | Promise<void>;
  }

  let {
    history = [],
    seasonal = [],
    trending = [],
    topRated = [],
    seasonalLoading = false,
    trendingLoading = false,
    topRatedLoading = false,
    seasonalMore = false,
    trendingMore = false,
    topRatedMore = false,
    getImage,
    onOpenSubject,
    onResumeHistory,
    onMoreSeasonal,
    onMoreTrending,
    onMoreTopRated,
  }: Props = $props();

  const continueItem = $derived(history[0] ?? null);
  const leadSubject = $derived(seasonal[0] ?? trending[0] ?? topRated[0] ?? null);
  const leadTitle = $derived(continueItem?.name || leadSubject?.name_cn || leadSubject?.name || "本季节目档案");
  const leadImage = $derived.by(() => {
    const source = continueItem?.image || leadSubject?.image || "";
    return source ? (getImage(source) || source) : "";
  });
  const leadMeta = $derived(continueItem
    ? `${continueItem.ruleName} / ${continueItem.lastEpisodeName || `第 ${continueItem.lastEpisode + 1} 集`}`
    : leadSubject
      ? [leadSubject.air_date, leadSubject.eps_count > 0 ? `${leadSubject.eps_count} 话` : "", leadSubject.rating > 0 ? `评分 ${leadSubject.rating.toFixed(1)}` : ""].filter(Boolean).join(" / ")
      : "等待节目数据");
  const leadDescription = $derived(leadSubject?.summary || "从最近观看位置继续，或进入节目档案选择剧集与播放来源。");
  const indexItems = $derived((trending.length ? trending : seasonal).slice(0, 12));
  const mosaicItems = $derived([...seasonal.slice(1, 5), ...topRated.slice(0, 4)].slice(0, 8));

  function openLead(event: MouseEvent) {
    const trigger = event.currentTarget as HTMLElement;
    if (continueItem) onResumeHistory(continueItem, trigger);
    else if (leadSubject) onOpenSubject(leadSubject, trigger);
  }

  function imageOf(subject: BangumiSubject): string {
    return subject.image ? getImage(subject.image) : "";
  }
</script>

<div class="editorial-home" data-testid="anime-editorial-home">
  <section class="editorial-lead" aria-labelledby="editorial-lead-title">
    <div class="lead-media" data-has-image={Boolean(leadImage)}>
      {#if leadImage}
        <img src={leadImage} alt="" loading="eager" decoding="async" />
      {:else}
        <div class="media-fallback" aria-hidden="true"><span>ANIME</span><strong>PROGRAM<br />ARCHIVE</strong></div>
      {/if}
      <span class="lead-grid" aria-hidden="true"></span>
    </div>
    <div class="lead-copy">
      <div class="lead-kicker"><span>01</span><i></i><b>{continueItem ? "CONTINUE WATCHING" : "SEASON PREMIERE"}</b></div>
      <p class="lead-meta">{leadMeta}</p>
      <h1 id="editorial-lead-title">{leadTitle}</h1>
      <p class="lead-description">{leadDescription}</p>
      <button class="lead-action" type="button" onclick={openLead} disabled={!continueItem && !leadSubject}>
        <span>{continueItem ? "继续观看" : "查看节目"}</span><i aria-hidden="true"></i>
      </button>
    </div>
    <aside class="lead-folio" aria-label="节目舞台信息">
      <span>MOEPLAY / EDITORIAL</span>
      <strong>{String(history.length).padStart(2, "0")}</strong>
      <small>WATCH RECORDS</small>
    </aside>
  </section>

  <section class="editorial-index" aria-labelledby="editorial-index-title">
    <header class="section-heading">
      <div><span>02 / TEXT INDEX</span><h2 id="editorial-index-title">热度节目索引</h2></div>
      <p>以标题、放送日期、评分和话数快速扫描节目。</p>
    </header>
    {#if indexItems.length}
      <div class="index-table">
        {#each indexItems as subject, index (subject.id)}
          <button type="button" onclick={(event) => onOpenSubject(subject, event.currentTarget)}>
            <span class="index-no">{String(index + 1).padStart(2, "0")}</span>
            <span class="index-title"><strong>{subject.name_cn || subject.name}</strong><small>{subject.name_cn && subject.name !== subject.name_cn ? subject.name : "BANGUMI PROGRAM"}</small></span>
            <span class="index-air">{subject.air_date || "日期未定"}</span>
            <span class="index-eps">{subject.eps_count > 0 ? `${subject.eps_count} 话` : "连载"}</span>
            <span class="index-score">{subject.rating > 0 ? subject.rating.toFixed(1) : "—"}</span>
            <span class="index-arrow" aria-hidden="true">↗</span>
          </button>
        {/each}
      </div>
    {:else}
      <div class="editorial-empty" role="status">{trendingLoading ? "正在编排节目索引…" : "暂无热度节目数据"}</div>
    {/if}
    {#if trendingMore && onMoreTrending}
      <button class="more-action" type="button" disabled={trendingLoading} onclick={() => void onMoreTrending?.()}>{trendingLoading ? "加载中…" : "展开更多热度节目"}</button>
    {/if}
  </section>

  <section class="editorial-mosaic" aria-labelledby="editorial-mosaic-title">
    <header class="section-heading section-heading--light">
      <div><span>03 / MEDIA SPLIT</span><h2 id="editorial-mosaic-title">本季媒体分割</h2></div>
      <p>真实节目封面以横幅、竖幅和局部裁切组成连续画报。</p>
    </header>
    {#if mosaicItems.length}
      <div class="mosaic-grid">
        {#each mosaicItems as subject, index (subject.id)}
          <button class={`mosaic-item mosaic-item--${index + 1}`} type="button" onclick={(event) => onOpenSubject(subject, event.currentTarget)}>
            {#if imageOf(subject)}<img src={imageOf(subject)} alt="" loading="lazy" decoding="async" />{:else}<span class="mosaic-fallback">NO MEDIA</span>{/if}
            <span class="mosaic-shade" aria-hidden="true"></span>
            <span class="mosaic-caption"><small>{String(index + 1).padStart(2, "0")} / {subject.air_date || "ARCHIVE"}</small><strong>{subject.name_cn || subject.name}</strong></span>
          </button>
        {/each}
      </div>
    {:else}
      <div class="editorial-empty editorial-empty--dark" role="status">{seasonalLoading || topRatedLoading ? "正在装配节目画报…" : "暂无可用节目媒体"}</div>
    {/if}
    <div class="mosaic-actions">
      {#if seasonalMore && onMoreSeasonal}<button type="button" disabled={seasonalLoading} onclick={() => void onMoreSeasonal?.()}>更多本季节目</button>{/if}
      {#if topRatedMore && onMoreTopRated}<button type="button" disabled={topRatedLoading} onclick={() => void onMoreTopRated?.()}>更多高分节目</button>{/if}
    </div>
  </section>
</div>

<style>
  .editorial-home{--ed-paper:#e8e3d8;--ed-ink:#101112;--ed-night:#080a0d;--ed-line:rgba(255,255,255,.16);--ed-accent:#ef5b43;display:grid;background:var(--ed-night);color:#f3f0e9;font-family:var(--font-ui,"Outfit","Noto Sans SC",sans-serif)}
  .editorial-lead{position:relative;display:grid;grid-template-columns:minmax(0,1.24fr) minmax(22rem,.76fr);min-height:clamp(34rem,68vh,48rem);overflow:hidden;border-bottom:1px solid var(--ed-line)}
  .lead-media{position:relative;min-height:34rem;background:#16181b;overflow:hidden}.lead-media img{width:100%;height:100%;display:block;object-fit:cover;object-position:center 28%;filter:saturate(.72) contrast(1.08);transform:scale(1.015)}
  .lead-media::after{content:"";position:absolute;inset:0;background:linear-gradient(90deg,transparent 58%,var(--ed-night) 100%),linear-gradient(0deg,rgba(0,0,0,.62),transparent 56%)}
  .lead-grid{position:absolute;z-index:2;inset:1.35rem;border:1px solid rgba(255,255,255,.25);pointer-events:none}.lead-grid::before,.lead-grid::after{content:"";position:absolute;background:rgba(255,255,255,.18)}.lead-grid::before{top:0;bottom:0;left:33%;width:1px}.lead-grid::after{left:0;right:0;bottom:21%;height:1px}
  .media-fallback{height:100%;display:grid;align-content:space-between;padding:3rem;background:linear-gradient(145deg,#24262a,#0c0e11);font-family:var(--font-mono,monospace)}.media-fallback span{font-size:.65rem;letter-spacing:.2em}.media-fallback strong{font:800 clamp(3rem,7vw,8rem)/.78 var(--font-display,"Outfit",sans-serif);letter-spacing:-.08em;color:#34373c}
  .lead-copy{position:relative;z-index:3;align-self:center;margin-left:clamp(-7rem,-6vw,-3rem);padding:4rem clamp(2rem,5vw,6rem) 4rem 0}.lead-kicker{display:flex;align-items:center;gap:.8rem;font:650 .62rem/1 var(--font-mono,monospace);letter-spacing:.16em}.lead-kicker span{color:var(--ed-accent)}.lead-kicker i{width:3rem;height:1px;background:currentColor}.lead-meta{margin:clamp(2rem,5vh,4rem) 0 .8rem;color:rgba(255,255,255,.56);font:600 .68rem/1.4 var(--font-mono,monospace);letter-spacing:.11em;text-transform:uppercase}.lead-copy h1{max-width:9ch;margin:0;font:800 clamp(3.6rem,7vw,7.8rem)/.82 var(--font-display,"Outfit",sans-serif);letter-spacing:-.075em;text-wrap:balance}.lead-description{max-width:37rem;margin:1.7rem 0 2rem;color:rgba(255,255,255,.65);font-size:.94rem;line-height:1.7;display:-webkit-box;overflow:hidden;line-clamp:3;-webkit-line-clamp:3;-webkit-box-orient:vertical}.lead-action{display:inline-flex;align-items:center;justify-content:space-between;gap:3rem;min-width:12rem;min-height:3rem;padding:0 1.1rem;border:1px solid var(--ed-accent);border-radius:0;background:var(--ed-accent);color:#0b0c0e;font-weight:750;cursor:pointer}.lead-action i{position:relative;width:2rem;height:1px;background:currentColor}.lead-action i::after{content:"";position:absolute;right:0;top:-3px;width:7px;height:7px;border-top:1px solid;border-right:1px solid;transform:rotate(45deg)}.lead-action:disabled{opacity:.45;cursor:not-allowed}.lead-folio{position:absolute;z-index:4;right:1.5rem;top:1.5rem;display:grid;justify-items:end;gap:.25rem;font-family:var(--font-mono,monospace)}.lead-folio span,.lead-folio small{font-size:.55rem;letter-spacing:.14em;color:rgba(255,255,255,.48)}.lead-folio strong{font-size:2.2rem;line-height:1}
  .editorial-index{padding:clamp(3rem,7vw,7rem);background:var(--ed-paper);color:var(--ed-ink)}.section-heading{display:grid;grid-template-columns:1fr minmax(16rem,.42fr);gap:2rem;align-items:end;margin-bottom:2.4rem;padding-bottom:1.25rem;border-bottom:1px solid currentColor}.section-heading span{font:700 .62rem/1 var(--font-mono,monospace);letter-spacing:.18em}.section-heading h2{margin:.65rem 0 0;font:800 clamp(2.5rem,5vw,5.6rem)/.86 var(--font-display,"Outfit",sans-serif);letter-spacing:-.065em}.section-heading p{justify-self:end;max-width:28rem;margin:0;font-size:.84rem;line-height:1.6;opacity:.62}.index-table{border-top:1px solid rgba(16,17,18,.25)}.index-table button{width:100%;display:grid;grid-template-columns:3.2rem minmax(12rem,1fr) 8rem 5rem 3rem 1.5rem;align-items:center;gap:1rem;min-height:4.7rem;padding:.65rem 0;border:0;border-bottom:1px solid rgba(16,17,18,.2);background:transparent;color:inherit;text-align:left;cursor:pointer;transition:padding 180ms ease,background 180ms ease}.index-table button:hover,.index-table button:focus-visible{padding-inline:1rem;background:#d8d1c3;outline:none}.index-no,.index-air,.index-eps,.index-score{font:650 .65rem/1.3 var(--font-mono,monospace)}.index-no{color:var(--ed-accent)}.index-title{display:grid;gap:.2rem;min-width:0}.index-title strong{overflow:hidden;font-size:1.05rem;text-overflow:ellipsis;white-space:nowrap}.index-title small{overflow:hidden;opacity:.45;font:600 .58rem/1.2 var(--font-mono,monospace);letter-spacing:.08em;text-overflow:ellipsis;white-space:nowrap}.index-score{font-size:1rem}.index-arrow{font-size:1.1rem}.more-action{margin-top:1.5rem;min-height:2.7rem;padding:0 1rem;border:1px solid currentColor;background:transparent;color:inherit;font-weight:700;cursor:pointer}
  .editorial-mosaic{padding:clamp(3rem,6vw,6rem);background:#090b0e}.section-heading--light{color:#f3f0e9;border-color:rgba(255,255,255,.3)}.mosaic-grid{display:grid;grid-template-columns:1.2fr .75fr .75fr 1fr;grid-template-rows:repeat(2,minmax(12rem,22vw));gap:1px;background:rgba(255,255,255,.18);border:1px solid rgba(255,255,255,.18)}.mosaic-item{position:relative;min-width:0;overflow:hidden;padding:0;border:0;background:#1a1c20;color:white;text-align:left;cursor:pointer}.mosaic-item--1{grid-row:1/3}.mosaic-item--2{grid-column:2/4}.mosaic-item img{width:100%;height:100%;display:block;object-fit:cover;filter:saturate(.72) contrast(1.08);transition:transform .55s cubic-bezier(.2,.8,.2,1),filter .2s ease}.mosaic-item:hover img,.mosaic-item:focus-visible img{transform:scale(1.035);filter:saturate(.9) contrast(1.04)}.mosaic-item:focus-visible{outline:2px solid var(--ed-accent);outline-offset:-2px}.mosaic-shade{position:absolute;inset:0;background:linear-gradient(0deg,rgba(4,5,7,.9),transparent 65%)}.mosaic-caption{position:absolute;right:1rem;bottom:1rem;left:1rem;display:grid;gap:.35rem}.mosaic-caption small{font:600 .55rem/1 var(--font-mono,monospace);letter-spacing:.1em;color:rgba(255,255,255,.55)}.mosaic-caption strong{font-size:clamp(.9rem,1.7vw,1.55rem);line-height:1.05}.mosaic-item--1 .mosaic-caption strong{font-size:clamp(1.8rem,3.7vw,4.3rem);letter-spacing:-.05em}.mosaic-fallback{height:100%;display:grid;place-items:center;color:rgba(255,255,255,.25);font:700 .65rem/1 var(--font-mono,monospace);letter-spacing:.15em}.mosaic-actions{display:flex;justify-content:flex-end;gap:.6rem;margin-top:1.2rem}.mosaic-actions button{min-height:2.5rem;padding:0 .9rem;border:1px solid rgba(255,255,255,.3);background:transparent;color:white;font-weight:650;cursor:pointer}.editorial-empty{padding:4rem 1rem;border-block:1px solid rgba(16,17,18,.22);font:650 .75rem/1 var(--font-mono,monospace);letter-spacing:.12em;text-align:center}.editorial-empty--dark{border-color:rgba(255,255,255,.2);color:rgba(255,255,255,.55)}
  @media(max-width:1000px){.editorial-lead{grid-template-columns:1fr;min-height:auto}.lead-media{height:58vh;min-height:28rem}.lead-copy{margin:-10rem 0 0;padding:3rem 2rem 4rem;background:linear-gradient(0deg,var(--ed-night) 76%,transparent)}.lead-copy h1{font-size:clamp(3.5rem,10vw,6rem)}.lead-folio{display:none}.index-table button{grid-template-columns:2.5rem minmax(0,1fr) 5rem 3rem}.index-air,.index-eps{display:none}.mosaic-grid{grid-template-columns:1.1fr .9fr;grid-template-rows:22rem 14rem 14rem}.mosaic-item--1{grid-row:1/3}.mosaic-item--2{grid-column:2;grid-row:1}.mosaic-item:nth-child(n+6){display:none}}
  @media(max-width:680px){.lead-media{height:52vh;min-height:24rem}.lead-copy{margin-top:-8rem;padding-inline:1rem}.lead-copy h1{font-size:clamp(3rem,17vw,5rem)}.lead-description{font-size:.86rem}.editorial-index,.editorial-mosaic{padding:3rem 1rem}.section-heading{grid-template-columns:1fr}.section-heading p{justify-self:start}.index-table button{grid-template-columns:2rem minmax(0,1fr) 2.5rem 1rem;gap:.5rem}.index-title small{display:none}.mosaic-grid{grid-template-columns:1fr 1fr;grid-template-rows:18rem 11rem}.mosaic-item--1{grid-column:1/3;grid-row:1}.mosaic-item--2{grid-column:1;grid-row:2}.mosaic-item:nth-child(n+4){display:none}.mosaic-item--1 .mosaic-caption strong{font-size:2rem}}
  @media(prefers-reduced-motion:reduce){.index-table button,.mosaic-item img{transition:none}.mosaic-item:hover img,.mosaic-item:focus-visible img{transform:none}}
</style>

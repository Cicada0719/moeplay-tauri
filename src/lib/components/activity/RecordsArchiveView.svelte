<script lang="ts">
  import type { DashboardChartPoint, DashboardMediaActivity, DashboardStat } from "./dashboard-model";

  let {
    items = [],
    stats = [],
    dailyPoints = [],
    monthlyPoints = [],
    warning = null,
    loading = false,
    onOpen,
    onImport,
  }: {
    items?: DashboardMediaActivity[];
    stats?: DashboardStat[];
    dailyPoints?: DashboardChartPoint[];
    monthlyPoints?: DashboardChartPoint[];
    warning?: string | null;
    loading?: boolean;
    onOpen: (item: DashboardMediaActivity) => void;
    onImport: () => void;
  } = $props();

  let activeId = $state<string | null>(null);
  const active = $derived(items.find((item) => item.id === activeId) ?? items[0] ?? null);
  const year = new Date().getFullYear();
  const maxDaily = $derived(Math.max(1, ...dailyPoints.map((item) => item.value)));
  const maxMonthly = $derived(Math.max(1, ...monthlyPoints.map((item) => item.value)));

  function kindLabel(kind: DashboardMediaActivity["kind"]) {
    return kind === "game" ? "GAME SESSION" : kind === "anime" ? "ANIME WATCH" : "COMIC READ";
  }

  function kindCn(kind: DashboardMediaActivity["kind"]) {
    return kind === "game" ? "游戏" : kind === "anime" ? "番剧" : "漫画";
  }

  function setActive(item: DashboardMediaActivity) {
    activeId = item.id;
  }
</script>

<section class="archive" aria-labelledby="archive-title" data-testid="records-archive">
  <header class="archive-cover">
    <div class="cover-register">
      <span>MOEPLAY / PERSONAL MEDIA</span>
      <span>{year} / ACTIVITY ARCHIVE</span>
    </div>
    <div class="cover-title">
      <p>游戏、番剧与漫画构成的私人媒体年鉴</p>
      <h1 id="archive-title"><span>ACTIVITY</span><span>ARCHIVE</span></h1>
    </div>
    <div class="cover-stats" aria-label="记录统计">
      {#each stats as stat, index (stat.id)}
        <article>
          <span>0{index + 1} / {stat.label}</span>
          <strong>{stat.value}</strong>
          {#if stat.detail}<p>{stat.detail}</p>{/if}
        </article>
      {/each}
    </div>
    <div class="cover-year" aria-hidden="true">{year}</div>
  </header>

  {#if warning}<p class="archive-warning" role="status">{warning}</p>{/if}

  {#if loading}
    <div class="archive-state" role="status">正在整理个人媒体档案…</div>
  {:else if items.length === 0}
    <div class="archive-state empty">
      <span>000 / EMPTY ARCHIVE</span>
      <h2>档案还没有内容</h2>
      <p>开始游玩、观看或阅读后，活动会出现在这里。</p>
      <button type="button" onclick={onImport}>导入游戏</button>
    </div>
  {:else}
    <div class="archive-body">
      <div class="archive-index" role="list" aria-label="个人媒体活动档案">
        {#each items.slice(0, 30) as item, index (item.id)}
          <div role="listitem">
          <button
            type="button"
            class:active={active?.id === item.id}
            onmouseenter={() => setActive(item)}
            onfocus={() => setActive(item)}
            onclick={() => onOpen(item)}
          >
            <span class="entry-no">{String(index + 1).padStart(3, "0")}</span>
            <span class="entry-kind">{kindLabel(item.kind)}</span>
            <span class="entry-title">{item.title}</span>
            <span class="entry-meta">{item.subtitle}</span>
            <time datetime={new Date(item.timestamp).toISOString()}>{item.timeLabel}</time>
            <span class="entry-arrow" aria-hidden="true">↗</span>
          </button>
          </div>
        {/each}
      </div>

      <aside class="media-stage" aria-live="polite">
        {#if active}
          <div class="stage-register"><span>{kindCn(active.kind)}</span><span>{active.timeLabel}</span></div>
          <div class="stage-image" class:no-image={!active.imageSrc}>
            {#if active.imageSrc}<img src={active.imageSrc} alt={`${active.title} 媒体封面`} />{:else}<span>{active.title.slice(0, 1)}</span>{/if}
          </div>
          <div class="stage-copy">
            <span>{kindLabel(active.kind)}</span>
            <h2>{active.title}</h2>
            <p>{active.subtitle}</p>
            <button type="button" onclick={() => onOpen(active)}>继续 / 打开记录</button>
          </div>
        {/if}
      </aside>
    </div>

    <section class="data-interlude" aria-labelledby="frequency-title">
      <header><span>DATA INSERT / 01</span><h2 id="frequency-title">最近 14 天活动频率</h2></header>
      <div class="frequency-list">
        {#each dailyPoints as point}
          <div><span>{point.label}</span><i style={`--bar:${Math.max(2, point.value / maxDaily * 100)}%`}></i><strong>{point.valueLabel}</strong></div>
        {/each}
      </div>
    </section>

    <section class="month-interlude" aria-labelledby="month-title">
      <header><span>DATA INSERT / 02</span><h2 id="month-title">月度媒体档案</h2></header>
      <div class="month-list">
        {#each monthlyPoints as point, index}
          <article style={`--weight:${Math.max(.15, point.value / maxMonthly)}`}>
            <span>{String(index + 1).padStart(2, "0")}</span><h3>{point.label}</h3><strong>{point.valueLabel}</strong>
          </article>
        {/each}
      </div>
    </section>
  {/if}
</section>

<style>
  .archive { --archive-paper:#eeeae0; --archive-muted:#9c998f; --archive-line:rgba(238,234,224,.18); --archive-accent:#c7472f; color:var(--archive-paper); background:#060606; }
  .archive-cover { position:relative; min-height:min(720px,calc(100dvh - 64px)); display:grid; grid-template-columns:minmax(0,1.2fr) minmax(18rem,.8fr); grid-template-rows:auto 1fr auto; gap:clamp(2rem,5vw,6rem); padding:clamp(24px,4vw,64px); overflow:hidden; border-bottom:1px solid var(--archive-line); }
  .cover-register { grid-column:1/-1; display:flex; justify-content:space-between; color:var(--archive-muted); font:600 10px/1 var(--font-mono); letter-spacing:.16em; }
  .cover-title { align-self:center; position:relative; z-index:1; }
  .cover-title p { max-width:28rem; margin:0 0 2rem; color:var(--archive-muted); font-size:clamp(.8rem,1vw,1rem); letter-spacing:.08em; }
  .cover-title h1 { margin:0; display:grid; font-size:clamp(4rem,11vw,11rem); line-height:.72; letter-spacing:-.075em; font-weight:650; }
  .cover-title h1 span:last-child { color:transparent; -webkit-text-stroke:1px var(--archive-paper); }
  .cover-stats { align-self:end; display:grid; grid-template-columns:repeat(2,minmax(0,1fr)); border-top:1px solid var(--archive-line); border-left:1px solid var(--archive-line); }
  .cover-stats article { min-height:9.5rem; padding:1rem; border-right:1px solid var(--archive-line); border-bottom:1px solid var(--archive-line); }
  .cover-stats span,.data-interlude header span,.month-interlude header span { color:var(--archive-accent); font:650 9px/1 var(--font-mono); letter-spacing:.14em; }
  .cover-stats strong { display:block; margin-top:1.3rem; font-size:clamp(1.8rem,4vw,4rem); line-height:1; letter-spacing:-.05em; }
  .cover-stats p { margin:.65rem 0 0; color:var(--archive-muted); font-size:.72rem; }
  .cover-year { position:absolute; right:-.04em; top:.3em; color:rgba(238,234,224,.035); font:700 clamp(12rem,35vw,36rem)/.7 var(--font-display); letter-spacing:-.1em; pointer-events:none; }
  .archive-warning { margin:0; padding:1rem clamp(24px,4vw,64px); border-bottom:1px solid var(--archive-line); color:#f0bf70; }
  .archive-body { min-height:100dvh; display:grid; grid-template-columns:minmax(0,1.05fr) minmax(25rem,.95fr); border-bottom:1px solid var(--archive-line); }
  .archive-index { border-right:1px solid var(--archive-line); }
  .archive-index > div { border-bottom:1px solid var(--archive-line); }
  .archive-index button { width:100%; min-height:112px; display:grid; grid-template-columns:56px 120px minmax(0,1fr) minmax(9rem,.4fr) 120px 20px; align-items:center; gap:1rem; padding:1rem clamp(20px,3vw,48px); border:0; background:transparent; color:var(--archive-muted); text-align:left; cursor:pointer; transition:background 180ms ease,color 180ms ease; }
  .archive-index button:hover,.archive-index button.active { color:var(--archive-paper); background:rgba(238,234,224,.035); }
  .archive-index button.active { box-shadow:inset 3px 0 var(--archive-accent); }
  .entry-no,.entry-kind,time { font:600 9px/1.3 var(--font-mono); letter-spacing:.1em; }
  .entry-kind { color:var(--archive-accent); }
  .entry-title { font-size:clamp(1.15rem,2vw,2rem); font-weight:600; letter-spacing:-.035em; }
  .entry-meta,time { color:var(--archive-muted); font-size:.7rem; }
  .media-stage { position:sticky; top:0; height:calc(100dvh - 64px); display:grid; grid-template-rows:auto minmax(0,1fr) auto; padding:clamp(20px,3vw,48px); overflow:hidden; }
  .stage-register { display:flex; justify-content:space-between; color:var(--archive-muted); font:600 9px/1 var(--font-mono); letter-spacing:.12em; }
  .stage-image { min-height:0; margin:clamp(20px,4vh,48px) 0; overflow:hidden; background:#101010; }
  .stage-image img { width:100%; height:100%; object-fit:cover; filter:saturate(.82) contrast(1.04); }
  .stage-image.no-image { display:grid; place-items:center; color:rgba(238,234,224,.12); font-size:14rem; }
  .stage-copy { display:grid; grid-template-columns:1fr auto; align-items:end; gap:.45rem 1rem; }
  .stage-copy span { grid-column:1/-1; color:var(--archive-accent); font:650 9px/1 var(--font-mono); letter-spacing:.14em; }
  .stage-copy h2 { margin:0; font-size:clamp(2rem,4vw,4.8rem); line-height:.86; letter-spacing:-.06em; }
  .stage-copy p { margin:.7rem 0 0; color:var(--archive-muted); }
  .stage-copy button,.archive-state button { grid-column:2; grid-row:2/4; align-self:end; min-height:44px; padding:0 1rem; border:1px solid var(--archive-paper); border-radius:0; background:transparent; color:var(--archive-paper); cursor:pointer; }
  .data-interlude,.month-interlude { display:grid; grid-template-columns:minmax(14rem,.36fr) 1fr; gap:clamp(2rem,6vw,8rem); padding:clamp(32px,6vw,88px) clamp(24px,4vw,64px); border-bottom:1px solid var(--archive-line); }
  .data-interlude header h2,.month-interlude header h2 { margin:1rem 0 0; font-size:clamp(2rem,4.5vw,5rem); line-height:.9; letter-spacing:-.055em; }
  .frequency-list { display:grid; }
  .frequency-list div { min-height:38px; display:grid; grid-template-columns:70px 1fr 90px; align-items:center; gap:1rem; border-bottom:1px solid var(--archive-line); }
  .frequency-list span,.frequency-list strong { font:500 9px/1 var(--font-mono); }
  .frequency-list strong { text-align:right; color:var(--archive-muted); }
  .frequency-list i { height:4px; width:var(--bar); background:var(--archive-accent); }
  .month-list { display:grid; grid-template-columns:repeat(2,minmax(0,1fr)); border-top:1px solid var(--archive-line); border-left:1px solid var(--archive-line); }
  .month-list article { min-height:160px; padding:1rem; border-right:1px solid var(--archive-line); border-bottom:1px solid var(--archive-line); background:rgba(199,71,47,calc(var(--weight)*.08)); }
  .month-list span { color:var(--archive-muted); font:600 9px/1 var(--font-mono); }
  .month-list h3 { margin:2.5rem 0 .5rem; font-size:clamp(1.6rem,3vw,3.3rem); letter-spacing:-.05em; }
  .month-list strong { color:var(--archive-muted); font:500 .8rem/1 var(--font-mono); }
  .archive-state { min-height:58vh; display:grid; place-items:center; align-content:center; gap:1rem; padding:3rem; text-align:center; }
  .archive-state span { color:var(--archive-accent); font:600 9px/1 var(--font-mono); letter-spacing:.14em; }.archive-state h2{margin:0;font-size:clamp(2rem,5vw,5rem)}.archive-state p{color:var(--archive-muted)}
  button:focus-visible { outline:1px solid var(--archive-paper); outline-offset:-3px; }
  @media(max-width:1100px){.archive-index button{grid-template-columns:46px 100px minmax(0,1fr) 100px 18px}.entry-meta{display:none}.archive-body{grid-template-columns:minmax(0,1fr) minmax(20rem,.75fr)}}
  @media(max-width:760px){.archive-cover{min-height:auto;grid-template-columns:1fr;padding:24px}.cover-stats{grid-column:1}.cover-title h1{font-size:clamp(3.6rem,20vw,7rem)}.archive-body{display:block}.media-stage{position:relative;height:auto;min-height:70dvh;border-bottom:1px solid var(--archive-line)}.archive-index button{grid-template-columns:38px 1fr 18px;min-height:92px}.entry-kind{grid-column:2}.entry-title{grid-column:2}.entry-meta,time{display:none}.entry-arrow{grid-column:3;grid-row:1/4}.data-interlude,.month-interlude{grid-template-columns:1fr}.month-list{grid-template-columns:1fr}.cover-year{font-size:18rem}}
  @media(prefers-reduced-motion:reduce){.archive-index button{transition:none}}
</style>

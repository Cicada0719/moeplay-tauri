<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { downloadStart, openUrl } from "../api";
  import { novelStore } from "../features/novel/store.svelte";
  import type { NovelBook, NovelChapter, NovelSource } from "../features/novel/types";
  import { uiStore } from "../stores/ui.svelte";
  import { i18n } from "../stores/i18n.svelte";
  import Icon from "./Icon.svelte";
  import { PageShell, PageHeader, FilterBar, AsyncState } from "./ui-v2";

  const READER_PREFS_KEY = "moeplay-novel-reader-prefs-v1";

  let searchInput = $state("");
  let readerElement = $state<HTMLElement | null>(null);
  let restoredReaderKey = $state("");
  let fontSize = $state(19);
  let lineHeight = $state(1.9);
  let readerTheme = $state<"dark" | "paper" | "sepia">("dark");
  let selectedSource = $state<NovelSource>(novelStore.source);
  let progressFrame = 0;
  let queuedSearchQuery = "";
  let queuedSearchVersion = 0;
  let completedSearchVersion = 0;
  let searchLoop: Promise<void> | null = null;

  const sourceOptions = $derived<Array<{ id: NovelSource; label: string; hint: string }>>([
    { id: "all", label: i18n.t("novel.source_all"), hint: i18n.t("novel.source_all_hint") },
    { id: "biquge", label: i18n.t("novel.source_biquge"), hint: i18n.t("novel.source_biquge_hint") },
    { id: "x80", label: i18n.t("novel.source_x80"), hint: i18n.t("novel.source_x80_hint") },
    { id: "internetarchive", label: i18n.t("novel.source_internetarchive"), hint: i18n.t("novel.source_internetarchive_hint") },
    { id: "openlibrary", label: i18n.t("novel.source_openlibrary"), hint: i18n.t("novel.source_openlibrary_hint") },
    { id: "standardebooks", label: i18n.t("novel.source_standardebooks"), hint: i18n.t("novel.source_standardebooks_hint") },
    { id: "gutenberg", label: i18n.t("novel.source_gutenberg"), hint: i18n.t("novel.source_gutenberg_hint") },
    { id: "wikisource", label: i18n.t("novel.source_wikisource"), hint: i18n.t("novel.source_wikisource_hint") },
  ]);

  const chapterIndex = $derived(
    novelStore.detail && novelStore.content
      ? novelStore.detail.chapters.findIndex((chapter) => chapter.id === novelStore.content?.chapter.id)
      : -1,
  );

  $effect(() => {
    const element = readerElement;
    const book = novelStore.detail?.book;
    const content = novelStore.content;
    if (!element || !book || !content) return;
    const key = `${book.source}:${book.id}:${content.chapter.id}`;
    if (restoredReaderKey === key) return;
    restoredReaderKey = key;
    const saved = novelStore.progressFor(book, content.chapter.id);
    requestAnimationFrame(() => {
      const available = Math.max(0, element.scrollHeight - element.clientHeight);
      element.scrollTop = available * saved;
    });
  });

  onMount(() => {
    try {
      const saved = JSON.parse(localStorage.getItem(READER_PREFS_KEY) ?? "{}") as {
        fontSize?: number;
        lineHeight?: number;
        theme?: "dark" | "paper" | "sepia";
      };
      if (typeof saved.fontSize === "number") fontSize = Math.max(15, Math.min(30, saved.fontSize));
      if (typeof saved.lineHeight === "number") lineHeight = Math.max(1.45, Math.min(2.5, saved.lineHeight));
      if (saved.theme === "dark" || saved.theme === "paper" || saved.theme === "sepia") readerTheme = saved.theme;
    } catch {
      // Invalid local preferences should never prevent opening the reader.
    }
    const handleKeydown = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      if (novelStore.view === "reader") {
        event.stopImmediatePropagation();
        novelStore.showDetail();
      } else if (novelStore.view === "detail") {
        event.stopImmediatePropagation();
        novelStore.showHome();
      }
    };
    window.addEventListener("keydown", handleKeydown, { capture: true });
    return () => window.removeEventListener("keydown", handleKeydown, { capture: true });
  });

  onDestroy(() => {
    if (progressFrame) cancelAnimationFrame(progressFrame);
    saveReaderProgress();
  });

  function persistReaderPrefs() {
    if (typeof localStorage === "undefined") return;
    localStorage.setItem(READER_PREFS_KEY, JSON.stringify({ fontSize, lineHeight, theme: readerTheme }));
  }

  async function drainSearchQueue() {
    while (completedSearchVersion < queuedSearchVersion) {
      const version = queuedSearchVersion;
      const query = queuedSearchQuery;
      const source = selectedSource;
      novelStore.setSource(source);
      await novelStore.search(query);
      completedSearchVersion = version;
    }
  }

  function queueSearch(query: string, source = selectedSource) {
    const normalized = query.trim();
    if (!normalized) return Promise.resolve();
    selectedSource = source;
    queuedSearchQuery = normalized;
    queuedSearchVersion += 1;
    if (!searchLoop) {
      searchLoop = drainSearchQueue().finally(() => {
        searchLoop = null;
      });
    }
    return searchLoop;
  }

  async function submitSearch(event?: SubmitEvent) {
    event?.preventDefault();
    await queueSearch(searchInput);
  }

  function selectSource(source: NovelSource) {
    selectedSource = source;
    if (searchInput.trim()) {
      void queueSearch(searchInput, source);
    } else {
      novelStore.setSource(source);
    }
  }

  function sourceLabel(source: NovelBook["source"]) {
    const labels: Record<NovelBook["source"], string> = {
      biquge: "novel.source_biquge_label",
      x80: "novel.source_x80_label",
      internetarchive: "novel.source_internetarchive_label",
      openlibrary: "novel.source_openlibrary_label",
      standardebooks: "novel.source_standardebooks_label",
      gutenberg: "novel.source_gutenberg_label",
      wikisource: "novel.source_wikisource_label",
    };
    return i18n.t(labels[source]);
  }

  function sourceBadge(source: NovelBook["source"]) {
    return { biquge: "BIQUGE", x80: "80XS", internetarchive: "ARCHIVE.ORG", openlibrary: "OPEN LIBRARY", standardebooks: "STANDARD EBOOKS", gutenberg: "GUTENBERG", wikisource: "WIKISOURCE" }[source];
  }

  function rightsKey(source: NovelBook["source"]) {
    return { biquge: "novel.rights_biquge", x80: "novel.rights_x80", internetarchive: "novel.rights_internetarchive", openlibrary: "novel.rights_openlibrary", standardebooks: "novel.rights_standardebooks", gutenberg: "novel.rights_gutenberg", wikisource: "novel.rights_wikisource" }[source];
  }

  function formatDate(timestamp: number) {
    return new Intl.DateTimeFormat(i18n.locale, { month: "2-digit", day: "2-digit" }).format(new Date(timestamp));
  }

  function sanitizeFilename(title: string, format?: string) {
    const extension = (format ?? "file").toLowerCase().replace(/[^a-z0-9]/g, "") || "file";
    let base = title
      .replace(/[\\/:*?"<>|\u0000-\u001f]/g, " ")
      .replace(/\s+/g, " ")
      .replace(/[. ]+$/g, "")
      .trim();
    if (!base || /^\.+$/.test(base)) base = "public-book";
    if (/^(con|prn|aux|nul|com[1-9]|lpt[1-9])$/i.test(base)) base = `_${base}`;
    const maxBaseLength = Math.max(1, 160 - extension.length - 1);
    base = Array.from(base).slice(0, maxBaseLength).join("").replace(/[. ]+$/g, "") || "public-book";
    return `${base}.${extension}`;
  }

  async function downloadBook(book: NovelBook) {
    if (!book.downloadUrl) return;
    try {
      await downloadStart(book.downloadUrl, sanitizeFilename(book.title, book.downloadFormat));
      uiStore.notify(i18n.t("novel.download_started", { format: book.downloadFormat ?? "FILE" }), "success");
    } catch (error) {
      uiStore.notify(i18n.t("novel.download_failed", { error: String(error) }), "error");
    }
  }

  async function openSource(book: NovelBook) {
    try {
      await openUrl(book.sourceUrl);
    } catch (error) {
      uiStore.notify(i18n.t("novel.open_source_failed", { error: String(error) }), "error");
    }
  }

  function saveReaderProgress() {
    const element = readerElement;
    if (!element || novelStore.view !== "reader") return;
    const available = element.scrollHeight - element.clientHeight;
    novelStore.setProgress(available <= 0 ? 1 : element.scrollTop / available);
  }

  function handleReaderScroll() {
    if (progressFrame) cancelAnimationFrame(progressFrame);
    progressFrame = requestAnimationFrame(() => {
      progressFrame = 0;
      saveReaderProgress();
    });
  }

  async function readChapter(chapter: NovelChapter) {
    saveReaderProgress();
    restoredReaderKey = "";
    await novelStore.readChapter(chapter);
  }

  async function moveChapter(offset: number) {
    const chapters = novelStore.detail?.chapters ?? [];
    const next = chapters[chapterIndex + offset];
    if (next) await readChapter(next);
  }
</script>

<PageShell as="div" width="full" scrollable={false} class="novel-v2-shell" ariaLabel={i18n.t("novel.title")}>
  <div class="novel-page" data-testid="novel-page">
    <div class="v2-grain nv-grain" aria-hidden="true"></div>

    {#if novelStore.view === "reader" && novelStore.detail && novelStore.content}
      <div class="reader-shell theme-{readerTheme}">
        <header class="reader-toolbar">
          <button type="button" class="icon-button" data-gamepad-activate="返回目录" aria-label={i18n.t("novel.back_to_catalog")} onclick={() => { saveReaderProgress(); novelStore.showDetail(); }}>
            <Icon name="arrowLeft" size={19} />
          </button>
          <div class="reader-identity">
            <small>{sourceLabel(novelStore.detail.book.source)}</small>
            <strong>{novelStore.detail.book.title}</strong>
            <span>{novelStore.content.chapter.title}</span>
          </div>
          <div class="reader-controls" aria-label={i18n.t("novel.controls_aria")}>
            <button type="button" aria-label={i18n.t("novel.font_decrease")} data-gamepad-activate="减小字号" disabled={fontSize <= 15} onclick={() => { fontSize -= 1; persistReaderPrefs(); }}>A−</button>
            <output aria-label={i18n.t("novel.font_size_aria")}>{fontSize}</output>
            <button type="button" aria-label={i18n.t("novel.font_increase")} data-gamepad-activate="增大字号" disabled={fontSize >= 30} onclick={() => { fontSize += 1; persistReaderPrefs(); }}>A+</button>
            <label>
              <span class="sr-only">{i18n.t("novel.line_height")}</span>
              <select aria-label={i18n.t("novel.line_height_aria")} bind:value={lineHeight} onchange={persistReaderPrefs}>
                <option value={1.55}>{i18n.t("novel.line_compact")}</option>
                <option value={1.9}>{i18n.t("novel.line_comfortable")}</option>
                <option value={2.2}>{i18n.t("novel.line_relaxed")}</option>
              </select>
            </label>
            <label>
              <span class="sr-only">{i18n.t("novel.theme_aria")}</span>
              <select aria-label={i18n.t("novel.theme_aria")} bind:value={readerTheme} onchange={persistReaderPrefs}>
                <option value="dark">{i18n.t("novel.theme_dark")}</option>
                <option value="paper">{i18n.t("novel.theme_paper")}</option>
                <option value="sepia">{i18n.t("novel.theme_sepia")}</option>
              </select>
            </label>
          </div>
        </header>

        <div
          class="reader-scroll"
          bind:this={readerElement}
          onscroll={handleReaderScroll}
          data-route-scroll
          style={`--reader-font-size:${fontSize}px;--reader-line-height:${lineHeight}`}
        >
          <article class="reader-article">
            <p class="reader-kicker">{sourceLabel(novelStore.content.source)} · {chapterIndex + 1} / {novelStore.detail.chapters.length}</p>
            <h1>{novelStore.content.chapter.title}</h1>
            <div class="reader-rule"></div>
            <div class="reader-prose">{novelStore.content.content}</div>
            <footer class="chapter-navigation" data-gamepad-group>
              <button type="button" data-gamepad-activate="上一章" disabled={chapterIndex <= 0 || novelStore.loading} onclick={() => moveChapter(-1)}>
                <Icon name="chevronLeft" size={17} /><span>{i18n.t("novel.prev_chapter")}</span>
              </button>
              <button type="button" data-gamepad-activate="返回目录" onclick={() => { saveReaderProgress(); novelStore.showDetail(); }}><span>{i18n.t("novel.back_to_catalog")}</span></button>
              <button type="button" data-gamepad-activate="下一章" disabled={chapterIndex < 0 || chapterIndex >= novelStore.detail.chapters.length - 1 || novelStore.loading} onclick={() => moveChapter(1)}>
                <span>{i18n.t("novel.next_chapter")}</span><Icon name="chevronRight" size={17} />
              </button>
            </footer>
          </article>
        </div>
      </div>

    {:else if novelStore.view === "detail" && novelStore.detail}
      <div class="novel-scroll detail-scroll" data-route-scroll>
        <header class="module-bar">
          <button type="button" class="back-button" data-gamepad-activate="返回书库" onclick={() => novelStore.showHome()}><Icon name="arrowLeft" size={17} />{i18n.t("novel.back_to_library")}</button>
          <span>NOVEL / CATALOG</span>
        </header>

        <section class="book-hero">
          <div class="hero-cover">
            {#if novelStore.detail.book.coverUrl}
              <img src={novelStore.detail.book.coverUrl} alt={i18n.t("novel.cover_alt", { title: novelStore.detail.book.title })} />
            {:else}
              <span><Icon name="collection" size={48} /><small>TEXT ARCHIVE</small></span>
            {/if}
          </div>
          <div class="hero-copy">
            <p class="eyebrow">{sourceLabel(novelStore.detail.book.source)}</p>
            <h1>{novelStore.detail.book.title}</h1>
            <p class="author">{novelStore.detail.book.author ?? i18n.t("novel.author_unknown")}</p>
            {#if novelStore.detail.book.summary}<p class="summary">{novelStore.detail.book.summary}</p>{/if}
            <div class="book-tags">
              {#if novelStore.detail.book.publicDomain}<span class="verified">{i18n.t("novel.public_domain")}</span>{/if}
              {#if novelStore.detail.book.language}<span>{novelStore.detail.book.language}</span>{/if}
              {#each novelStore.detail.book.subjects.slice(0, 6) as subject}<span>{subject}</span>{/each}
            </div>
            <div class="hero-actions" data-gamepad-group>
              {#if novelStore.detail.chapters[0]}
                <button class="primary-action" type="button" data-gamepad-activate="开始阅读" disabled={novelStore.loading} onclick={() => readChapter(novelStore.detail!.chapters[0])}>
                  <Icon name="book" size={17} />{i18n.t("novel.start_reading")}
                </button>
              {/if}
              {#if novelStore.detail.book.downloadUrl}
                <button type="button" data-gamepad-secondary-action data-gamepad-activate={i18n.t("novel.download_file", { format: novelStore.detail.book.downloadFormat ?? "FILE" })} onclick={() => downloadBook(novelStore.detail!.book)}><Icon name="download" size={17} />{i18n.t("novel.download_file", { format: novelStore.detail.book.downloadFormat ?? "FILE" })}</button>
              {/if}
              <button type="button" data-gamepad-activate="打开原文" onclick={() => openSource(novelStore.detail!.book)}><Icon name="externalLink" size={17} />{i18n.t("novel.view_source")}</button>
            </div>
            <p class="rights-note">{i18n.t(rightsKey(novelStore.detail.book.source))}</p>
          </div>
        </section>

        <section class="catalog-section">
          <div class="section-heading"><div><span>CONTENTS</span><h2>{i18n.t("novel.catalog_title")}</h2></div><p>{i18n.t("novel.chapter_count", { count: novelStore.detail.chapters.length })}</p></div>
          <div class="chapter-list">
            {#if novelStore.detail.chapters.length === 0}
              <div class="download-only"><Icon name="download" size={22} /><p>{i18n.t("novel.download_only")}</p></div>
            {/if}
            {#each novelStore.detail.chapters as chapter, index (chapter.id)}
              <button type="button" data-gamepad-label={`阅读 ${chapter.title}`} data-gamepad-activate="开始阅读" disabled={novelStore.loading} onclick={() => readChapter(chapter)}>
                <span class="chapter-number">{String(index + 1).padStart(3, "0")}</span>
                <span class="chapter-title">{chapter.title}</span>
                {#if novelStore.progressFor(novelStore.detail.book, chapter.id) > 0}
                  <span class="chapter-progress">{Math.round(novelStore.progressFor(novelStore.detail.book, chapter.id) * 100)}%</span>
                {/if}
                <Icon name="chevronRight" size={16} />
              </button>
            {/each}
          </div>
        </section>
      </div>

    {:else}
      <div class="novel-scroll home-scroll" data-route-scroll>
        <PageHeader
          id="novel-page-title"
          class="nv-header"
          eyebrow="小説 / NOVEL"
          title={i18n.t("novel.title")}
          description={i18n.t("novel.subtitle")}
        />

        <FilterBar label={i18n.t("novel.search_aria")} class="nv-searchbar">
          <form class="novel-search" onsubmit={submitSearch}>
            <Icon name="search" size={18} />
            <input type="search" bind:value={searchInput} placeholder={i18n.t("novel.search_placeholder")} aria-label={i18n.t("novel.search_aria")} data-search-scope="novel" data-gamepad-nav-down="#novel-source-tab-all" />
            <button type="submit" disabled={!searchInput.trim() || novelStore.loading}>{novelStore.loading ? i18n.t("novel.searching") : i18n.t("novel.search_submit")}</button>
          </form>
        </FilterBar>

        <div class="source-tabs" role="tablist" aria-label={i18n.t("novel.sources_aria")}>
          {#each sourceOptions as source (source.id)}
            <button type="button" role="tab" id={`novel-source-tab-${source.id}`} tabindex={selectedSource === source.id ? 0 : -1} data-gamepad-activate="切换来源" aria-selected={selectedSource === source.id} class:active={selectedSource === source.id} onclick={() => selectSource(source.id)}>
              <strong>{source.label}</strong><span>{source.hint}</span>
            </button>
          {/each}
        </div>

        {#if novelStore.error}
          <div class="error-banner" role="alert"><Icon name="info" size={18} /><span>{novelStore.error}</span><button type="button" onclick={() => novelStore.search()}>{i18n.t("button.retry")}</button></div>
        {/if}

        {#if novelStore.history.length > 0 && novelStore.books.length === 0}
          <section class="history-section">
            <div class="section-heading"><div><span>READING HISTORY</span><h2>{i18n.t("novel.history_title")}</h2></div><p>{i18n.t("novel.history_note")}</p></div>
            <div class="history-row">
              {#each novelStore.history.slice(0, 8) as entry (entry.key)}
                <button type="button" data-gamepad-label={`继续阅读 ${entry.book.title}`} data-gamepad-activate="继续阅读" onclick={() => novelStore.resume(entry)}>
                  <div class="history-cover">
                    {#if entry.book.coverUrl}<img src={entry.book.coverUrl} alt="" />{:else}<Icon name="book" size={25} />{/if}
                  </div>
                  <span class="history-copy"><strong>{entry.book.title}</strong><small>{entry.chapterTitle} · {formatDate(entry.updatedAt)}</small></span>
                  <span class="history-percent">{Math.round(entry.progress * 100)}%</span>
                  <span class="progress-track"><i style={`width:${Math.max(2, entry.progress * 100)}%`}></i></span>
                </button>
              {/each}
            </div>
          </section>
        {/if}

        {#if novelStore.loading && novelStore.books.length === 0}
          <AsyncState state="loading" loadingRows={4} title={i18n.t("novel.loading_title")} description={i18n.t("novel.loading_desc")} />
        {:else if novelStore.books.length > 0}
          <section class="results-section">
            <div class="section-heading"><div><span>SEARCH RESULTS</span><h2>“{novelStore.query}”</h2></div><p>{i18n.t("novel.results_count", { count: novelStore.books.length })}</p></div>
            <div class="book-grid">
              {#each novelStore.books as book (`${book.source}:${book.id}`)}
                <button type="button" class="book-card" data-gamepad-activate="打开图书详情" onclick={() => novelStore.openBook(book)} aria-label={i18n.t("novel.view_book_aria", { title: book.title })}>
                  <span class="card-cover">
                    {#if book.coverUrl}<img src={book.coverUrl} alt="" loading="lazy" />{:else}<Icon name="collection" size={38} />{/if}
                    <small>{sourceBadge(book.source)}</small>
                  </span>
                  <span class="card-copy">
                    <strong>{book.title}</strong>
                    <small>{book.author ?? sourceLabel(book.source)}</small>
                    {#if book.summary}<span>{book.summary}</span>{/if}
                    <i>{book.publicDomain ? i18n.t("novel.public_domain") : i18n.t("novel.free_text")}<Icon name="arrowRight" size={14} /></i>
                  </span>
                </button>
              {/each}
            </div>
          </section>
        {:else if novelStore.query}
          <AsyncState state="no-results" title={i18n.t("novel.no_results_title")} description={i18n.t("novel.no_results_desc")} />
        {:else}
          <section class="source-intro">
            <article><span>01</span><div><h2>{i18n.t("novel.intro_biquge_title")}</h2><p>{i18n.t("novel.intro_biquge_desc")}</p><small>{i18n.t("novel.intro_biquge_meta")}</small></div></article>
            <article><span>02</span><div><h2>{i18n.t("novel.intro_x80_title")}</h2><p>{i18n.t("novel.intro_x80_desc")}</p><small>{i18n.t("novel.intro_x80_meta")}</small></div></article>
            <article><span>03</span><div><h2>Project Gutenberg</h2><p>{i18n.t("novel.intro_gutenberg_desc")}</p><small>{i18n.t("novel.intro_gutenberg_meta")}</small></div></article>
            <article><span>04</span><div><h2>{i18n.t("novel.intro_wikisource_title")}</h2><p>{i18n.t("novel.intro_wikisource_desc")}</p><small>{i18n.t("novel.intro_wikisource_meta")}</small></div></article>
            <article><span>05</span><div><h2>Internet Archive</h2><p>{i18n.t("novel.intro_internetarchive_desc")}</p><small>{i18n.t("novel.intro_internetarchive_meta")}</small></div></article>
            <article><span>06</span><div><h2>Open Library</h2><p>{i18n.t("novel.intro_openlibrary_desc")}</p><small>{i18n.t("novel.intro_openlibrary_meta")}</small></div></article>
            <article><span>07</span><div><h2>Standard Ebooks</h2><p>{i18n.t("novel.intro_standardebooks_desc")}</p><small>{i18n.t("novel.intro_standardebooks_meta")}</small></div></article>
            <aside><Icon name="shield" size={20} /><p>{i18n.t("novel.intro_legal")}</p></aside>
          </section>
        {/if}
      </div>
    {/if}

    {#if novelStore.loading && novelStore.view !== "home"}
      <div class="detail-loading" role="status"><span class="spinner"></span><span>{i18n.t("novel.detail_loading")}</span></div>
    {/if}
  </div>
</PageShell>

<style>
  :global(.novel-v2-shell) { height: 100%; }
  :global(.novel-v2-shell .v2-page-shell__inner) { height: 100%; padding: 0; }

  .novel-page { position: relative; width: 100%; height: 100%; min-width: 0; min-height: 0; overflow: hidden; color: var(--text-primary); background: radial-gradient(circle at 18% 4%, rgba(183,136,103,.11), transparent 35%), #080807; }

  /* Halftone grain background layer (utility class lives in tokens-v2.css). */
  .nv-grain { position: absolute; inset: 0; z-index: 0; }

  .novel-scroll { position: relative; z-index: 1; height: 100%; overflow: auto; overscroll-behavior: contain; scrollbar-gutter: stable; }
  .home-scroll, .detail-scroll { padding: clamp(18px, 3.5vw, 54px); padding-bottom: max(44px, env(safe-area-inset-bottom)); }
  button, input, select { font: inherit; }
  button { color: inherit; }
  .sr-only { position: absolute; width: 1px; height: 1px; padding: 0; overflow: hidden; clip: rect(0,0,0,0); white-space: nowrap; border: 0; }

  :global(.nv-header) { padding-top: clamp(14px, 2vw, 24px); border-top: 1px solid rgba(255,255,255,.18); }
  :global(.nv-header .v2-page-header__title) { font-family: var(--font-display); letter-spacing: -.03em; }
  :global(.nv-searchbar) { margin-top: clamp(18px, 3vw, 32px); background: rgba(255,255,255,.02); border-color: rgba(255,255,255,.14); border-radius: 0; }
  .eyebrow, .module-bar > span, .section-heading span, .reader-kicker { color: #c69b7a; font: 700 9px/1.2 var(--font-mono); letter-spacing: .18em; }
  .novel-search { width: 100%; min-height: 44px; display: grid; grid-template-columns: auto minmax(0, 1fr) auto; align-items: center; gap: 10px; }
  .novel-search :global(.icon) { color: #c69b7a; }
  .novel-search input { width: 100%; height: 44px; padding: 0 6px; border: 0; outline: 0; color: var(--text-primary); background: transparent; font-size: 15px; }
  .novel-search button, .primary-action { min-height: 42px; padding: 0 20px; border: 1px solid #c69b7a; background: #c69b7a; color: #100d0b; font-weight: 750; cursor: pointer; }
  button:disabled { opacity: .38; cursor: not-allowed; }

  .source-tabs { display: grid; grid-template-columns: repeat(auto-fit, minmax(150px, 1fr)); margin: 22px 0 clamp(30px, 5vw, 68px); border: 1px solid rgba(255,255,255,.14); }
  .source-tabs button { min-height: 72px; display: grid; gap: 6px; padding: 14px 18px; border: 0; border-right: 1px solid rgba(255,255,255,.14); background: rgba(255,255,255,.015); text-align: left; cursor: pointer; }
  .source-tabs button:last-child { border-right: 0; }
  .source-tabs button.active { background: rgba(198,155,122,.13); box-shadow: inset 0 -2px #c69b7a; }
  .source-tabs strong { font-size: 13px; }
  .source-tabs span { color: var(--text-muted); font-size: 10px; line-height: 1.4; }

  .section-heading { display: flex; align-items: end; justify-content: space-between; gap: 20px; margin-bottom: 18px; }
  .section-heading h2 { margin: 7px 0 0; font: 550 clamp(25px, 3vw, 42px)/1 var(--font-display); letter-spacing: -.04em; }
  .section-heading p { margin: 0; color: var(--text-muted); font: 600 10px/1 var(--font-mono); }
  .history-section { margin-bottom: clamp(36px, 6vw, 78px); }
  .history-row { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 1px; background: rgba(255,255,255,.13); border: 1px solid rgba(255,255,255,.13); }
  .history-row > button { position: relative; min-width: 0; min-height: 108px; display: grid; grid-template-columns: 58px minmax(0, 1fr) auto; align-items: center; gap: 12px; padding: 14px; border: 0; background: #0d0c0b; text-align: left; cursor: pointer; }
  .history-cover { width: 58px; height: 76px; display: grid; place-items: center; overflow: hidden; background: #1c1815; color: #c69b7a; }
  .history-cover img { width: 100%; height: 100%; object-fit: cover; }
  .history-copy { min-width: 0; display: grid; gap: 8px; }
  .history-copy strong, .history-copy small { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .history-copy strong { font-size: 13px; }
  .history-copy small { color: var(--text-muted); font-size: 10px; }
  .history-percent { color: #c69b7a; font: 700 11px/1 var(--font-mono); }
  .progress-track { position: absolute; left: 84px; right: 14px; bottom: 13px; height: 2px; overflow: hidden; background: rgba(255,255,255,.09); }
  .progress-track i { display: block; height: 100%; background: #c69b7a; }

  .book-grid { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 1px; background: rgba(255,255,255,.13); border: 1px solid rgba(255,255,255,.13); }
  .book-card { min-width: 0; min-height: 300px; display: grid; grid-template-columns: minmax(100px, 42%) minmax(0, 1fr); gap: 0; padding: 0; border: 0; background: #0d0c0b; text-align: left; cursor: pointer; }
  .book-card:hover { background: #151210; }
  .card-cover { position: relative; min-height: 300px; display: grid; place-items: center; overflow: hidden; background: linear-gradient(145deg, #2b221d, #110f0d); color: #c69b7a; }
  .card-cover img { width: 100%; height: 100%; object-fit: cover; }
  .card-cover small { position: absolute; left: 10px; bottom: 10px; padding: 5px 7px; background: rgba(0,0,0,.78); color: #d9b99f; font: 700 7px/1 var(--font-mono); letter-spacing: .1em; }
  .card-copy { min-width: 0; display: flex; flex-direction: column; padding: 22px 16px; }
  .card-copy > strong { font: 600 clamp(16px, 1.45vw, 23px)/1.12 var(--font-display); letter-spacing: -.025em; overflow-wrap: anywhere; }
  .card-copy > small { margin-top: 9px; color: #c69b7a; font-size: 10px; line-height: 1.45; }
  .card-copy > span { display: -webkit-box; margin-top: 18px; overflow: hidden; color: var(--text-muted); font-size: 11px; line-height: 1.65; line-clamp: 5; -webkit-line-clamp: 5; -webkit-box-orient: vertical; }
  .card-copy > i { display: flex; align-items: center; justify-content: space-between; gap: 8px; margin-top: auto; padding-top: 16px; border-top: 1px solid rgba(255,255,255,.1); color: var(--text-secondary); font: normal 700 9px/1 var(--font-mono); }

  .source-intro { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 1px; background: rgba(255,255,255,.13); border: 1px solid rgba(255,255,255,.13); }
  .source-intro article { min-height: 230px; display: grid; grid-template-columns: auto 1fr; gap: 24px; padding: clamp(24px, 3vw, 46px); background: #0d0c0b; }
  .source-intro article > span { color: #c69b7a; font: 700 11px/1 var(--font-mono); }
  .source-intro h2 { margin: 0 0 18px; font: 550 clamp(22px, 3vw, 40px)/1 var(--font-display); }
  .source-intro p { margin: 0; color: var(--text-muted); font-size: 13px; line-height: 1.8; }
  .source-intro small { display: block; margin-top: 22px; color: #c69b7a; font: 700 9px/1.4 var(--font-mono); }
  .source-intro aside { grid-column: 1 / -1; display: flex; align-items: center; gap: 12px; padding: 16px 20px; background: rgba(198,155,122,.08); color: var(--text-secondary); }
  .source-intro aside p { margin: 0; font-size: 11px; line-height: 1.6; }
  .source-intro aside :global(.icon) { color: #c69b7a; }

  .error-banner { display: flex; align-items: center; gap: 10px; margin-bottom: 24px; padding: 13px 15px; border: 1px solid rgba(239,93,93,.5); color: #f2b2b2; background: rgba(128,22,22,.18); font-size: 12px; }
  .error-banner span { flex: 1; min-width: 0; overflow-wrap: anywhere; }
  .error-banner button { min-height: 34px; padding: 0 13px; border: 1px solid currentColor; background: transparent; cursor: pointer; }
  .spinner { width: 22px; height: 22px; border: 2px solid rgba(198,155,122,.25); border-top-color: #c69b7a; border-radius: 50%; animation: spin .8s linear infinite; }

  .module-bar { min-height: 50px; display: flex; justify-content: space-between; align-items: center; border-top: 1px solid rgba(255,255,255,.18); border-bottom: 1px solid rgba(255,255,255,.18); }
  .back-button, .hero-actions button { display: inline-flex; align-items: center; justify-content: center; gap: 8px; min-height: 40px; padding: 0 14px; border: 1px solid rgba(255,255,255,.18); background: transparent; cursor: pointer; }
  .book-hero { display: grid; grid-template-columns: minmax(220px, 330px) minmax(0, 1fr); gap: clamp(30px, 6vw, 100px); padding: clamp(36px, 6vw, 86px) 0; border-bottom: 1px solid rgba(255,255,255,.18); }
  .hero-cover { aspect-ratio: 2/3; display: grid; place-items: center; overflow: hidden; border: 1px solid rgba(255,255,255,.15); background: linear-gradient(145deg, #2b221d, #0e0c0a); }
  .hero-cover img { width: 100%; height: 100%; object-fit: cover; }
  .hero-cover > span { display: grid; justify-items: center; gap: 18px; color: #c69b7a; }
  .hero-cover small { font: 700 8px/1 var(--font-mono); letter-spacing: .14em; }
  .hero-copy { align-self: center; }
  .hero-copy h1 { max-width: 18ch; margin: 13px 0 14px; font: 520 clamp(38px, 6vw, 86px)/.96 var(--font-display); letter-spacing: -.06em; overflow-wrap: anywhere; }
  .author { margin: 0; color: #c69b7a; font-size: 13px; }
  .summary { max-width: 78ch; margin: 28px 0 0; color: var(--text-secondary); font-size: 13px; line-height: 1.85; }
  .book-tags { display: flex; flex-wrap: wrap; gap: 7px; margin-top: 23px; }
  .book-tags span { padding: 5px 8px; border: 1px solid rgba(255,255,255,.16); color: var(--text-muted); font-size: 10px; }
  .book-tags span.verified { border-color: rgba(124,190,146,.5); color: #9dd3af; }
  .hero-actions { display: flex; flex-wrap: wrap; gap: 8px; margin-top: 30px; }
  .hero-actions .primary-action { border-color: #c69b7a; background: #c69b7a; }
  .rights-note { max-width: 70ch; margin: 16px 0 0; color: var(--text-muted); font-size: 10px; line-height: 1.6; }
  .catalog-section { padding: clamp(36px, 5vw, 72px) 0; }
  .download-only { min-height: 120px; display: flex; align-items: center; justify-content: center; gap: 12px; padding: 24px; color: var(--text-muted); text-align: center; }
  .download-only p { margin: 0; max-width: 520px; line-height: 1.6; }
  .chapter-list { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); border-top: 1px solid rgba(255,255,255,.15); }
  .chapter-list button { min-width: 0; min-height: 62px; display: grid; grid-template-columns: 46px minmax(0, 1fr) auto auto; align-items: center; gap: 12px; padding: 8px 14px 8px 0; border: 0; border-bottom: 1px solid rgba(255,255,255,.12); background: transparent; text-align: left; cursor: pointer; }
  .chapter-list button:nth-child(odd) { padding-right: 26px; border-right: 1px solid rgba(255,255,255,.12); }
  .chapter-list button:nth-child(even) { padding-left: 20px; }
  .chapter-number, .chapter-progress { color: #c69b7a; font: 700 9px/1 var(--font-mono); }
  .chapter-title { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 12px; }

  .reader-shell { position: relative; z-index: 1; width: 100%; height: 100%; display: grid; grid-template-rows: auto minmax(0, 1fr); background: var(--reader-bg); color: var(--reader-text); --reader-bg: #090909; --reader-panel: rgba(12,12,12,.96); --reader-text: #ddd9d0; --reader-muted: #8c8982; --reader-line: rgba(255,255,255,.13); --reader-accent: #c69b7a; }
  .reader-shell.theme-paper { --reader-bg: #ebe8df; --reader-panel: rgba(237,234,226,.96); --reader-text: #282621; --reader-muted: #716c63; --reader-line: rgba(30,25,20,.17); --reader-accent: #875b3e; }
  .reader-shell.theme-sepia { --reader-bg: #d9c9aa; --reader-panel: rgba(220,204,174,.96); --reader-text: #34291d; --reader-muted: #77634b; --reader-line: rgba(60,42,25,.18); --reader-accent: #794b2e; }
  .reader-toolbar { z-index: 2; min-height: 66px; display: grid; grid-template-columns: auto minmax(0, 1fr) auto; align-items: center; gap: 14px; padding: max(8px, env(safe-area-inset-top)) max(14px, env(safe-area-inset-right)) 8px max(14px, env(safe-area-inset-left)); border-bottom: 1px solid var(--reader-line); background: var(--reader-panel); backdrop-filter: blur(16px); }
  .icon-button { width: 44px; height: 44px; display: grid; place-items: center; border: 1px solid var(--reader-line); background: transparent; cursor: pointer; }
  .reader-identity { min-width: 0; display: grid; gap: 3px; }
  .reader-identity small { color: var(--reader-accent); font: 700 7px/1 var(--font-mono); letter-spacing: .12em; text-transform: uppercase; }
  .reader-identity strong, .reader-identity span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .reader-identity strong { font-size: 13px; }
  .reader-identity span { color: var(--reader-muted); font-size: 10px; }
  .reader-controls { display: flex; align-items: center; gap: 5px; }
  .reader-controls button, .reader-controls select { min-height: 36px; border: 1px solid var(--reader-line); background: transparent; color: inherit; }
  .reader-controls button { min-width: 38px; padding: 0 8px; cursor: pointer; }
  .reader-controls output { min-width: 26px; color: var(--reader-muted); font: 700 10px/1 var(--font-mono); text-align: center; }
  .reader-controls select { padding: 0 8px; }
  .reader-controls option { background: #181818; color: #eee; }
  .reader-scroll { min-height: 0; overflow: auto; overscroll-behavior: contain; scroll-behavior: smooth; }
  .reader-article { width: min(100% - 36px, 820px); min-height: 100%; margin: 0 auto; padding: clamp(48px, 8vh, 100px) 0 max(80px, env(safe-area-inset-bottom)); }
  .reader-kicker { margin: 0 0 16px; color: var(--reader-accent); }
  .reader-article h1 { margin: 0; font: 550 clamp(28px, 4vw, 50px)/1.12 var(--font-display); letter-spacing: -.04em; }
  .reader-rule { width: 74px; height: 2px; margin: 28px 0 42px; background: var(--reader-accent); }
  .reader-prose { white-space: pre-wrap; color: var(--reader-text); font-family: "Noto Serif SC", "Source Han Serif SC", "Songti SC", SimSun, serif; font-size: var(--reader-font-size); line-height: var(--reader-line-height); letter-spacing: .018em; overflow-wrap: anywhere; }
  .chapter-navigation { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); margin-top: 80px; border-top: 1px solid var(--reader-line); border-bottom: 1px solid var(--reader-line); }
  .chapter-navigation button { min-height: 58px; display: flex; align-items: center; justify-content: center; gap: 8px; border: 0; border-right: 1px solid var(--reader-line); background: transparent; cursor: pointer; }
  .chapter-navigation button:last-child { border-right: 0; }

  .detail-loading { position: absolute; inset: 0; z-index: 9; display: flex; align-items: center; justify-content: center; gap: 12px; background: rgba(5,5,5,.72); backdrop-filter: blur(8px); }
  @keyframes spin { to { transform: rotate(360deg); } }

  @media (max-width: 1180px) {
    .book-grid { grid-template-columns: repeat(2, minmax(0, 1fr)); }
    .history-row { grid-template-columns: repeat(2, minmax(0, 1fr)); }
  }
  @media (max-width: 760px) {
    .home-scroll, .detail-scroll { padding: 14px; padding-bottom: max(24px, env(safe-area-inset-bottom)); }
    .source-tabs { grid-template-columns: 1fr; margin: 16px 0 34px; }
    .source-tabs button { min-height: 60px; border-right: 0; border-bottom: 1px solid rgba(255,255,255,.14); }
    .source-tabs button:last-child { border-bottom: 0; }
    .history-row, .book-grid, .source-intro, .chapter-list { grid-template-columns: 1fr; }
    .book-card { min-height: 240px; grid-template-columns: 116px minmax(0, 1fr); }
    .card-cover { min-height: 240px; }
    .source-intro aside { grid-column: auto; }
    .book-hero { grid-template-columns: 112px minmax(0, 1fr); gap: 18px; padding: 28px 0; }
    .hero-copy h1 { font-size: clamp(27px, 9vw, 50px); }
    .summary { grid-column: 1 / -1; font-size: 12px; }
    .hero-actions { grid-column: 1 / -1; }
    .hero-actions button { flex: 1 1 auto; }
    .chapter-list button:nth-child(odd), .chapter-list button:nth-child(even) { padding: 8px 10px 8px 0; border-right: 0; }
    .reader-toolbar { grid-template-columns: auto minmax(0, 1fr); }
    .reader-controls { grid-column: 1 / -1; justify-content: flex-end; overflow-x: auto; padding-top: 7px; border-top: 1px solid var(--reader-line); }
    .reader-article { width: min(100% - 28px, 720px); padding-top: 46px; }
  }
  @media (orientation: landscape) and (max-height: 600px) {
    .home-scroll, .detail-scroll { padding-top: max(12px, env(safe-area-inset-top)); padding-right: max(14px, env(safe-area-inset-right)); padding-bottom: max(14px, env(safe-area-inset-bottom)); }
    .source-tabs { grid-template-columns: repeat(2, minmax(0, 1fr)); margin: 12px 0 24px; }
    .source-tabs button { min-height: 54px; border-right: 1px solid rgba(255,255,255,.14); border-bottom: 0; }
    .source-tabs span { display: none; }
    .reader-toolbar { grid-template-columns: auto minmax(0, 1fr) auto; padding-top: max(6px, env(safe-area-inset-top)); }
    .reader-controls { grid-column: auto; border-top: 0; padding-top: 0; }
    .reader-article { padding-top: 36px; }
  }

  /* ── Reduced motion ── */
  @media (prefers-reduced-motion: reduce) {
    .spinner { animation: none; }
    .reader-scroll { scroll-behavior: auto; }
  }
  :global([data-motion="reduce"]) .spinner { animation: none; }
  :global([data-motion="reduce"]) .reader-scroll { scroll-behavior: auto; }
</style>

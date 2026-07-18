<script lang="ts">
  import { gameStore } from "../stores/games.svelte";
  import { i18n } from "../stores/i18n.svelte";
  import { uiStore } from "../stores/ui.svelte";
  import { searchEmulators, scanRoms, importRomGame, pickDirectory, type ScannedEmulator, type RomFile } from "../api";
  import Icon from "./Icon.svelte";
  import Button from "./ui/Button.svelte";
  import { PageShell, PageHeader, AsyncState, type ViewState } from "./ui-v2";

  let scanning = $state(false);
  let emulators = $state<ScannedEmulator[]>([]);
  let selectedEmu = $state<ScannedEmulator | null>(null);
  let selectedProfileIdx = $state(0);

  let romDir = $state("");
  let roms = $state<RomFile[]>([]);
  let scanningRoms = $state(false);
  let selectedRoms = $state<Set<number>>(new Set());

  let importing = $state(false);
  let importResult = $state<{ ok: number; fail: number } | null>(null);

  const defaultPaths = [
    "C:\\Program Files",
    "C:\\Program Files (x86)",
    // Don't scan entire drives — they have millions of files
  ];

  let hasScanned = $state(false);

  async function doScanEmulators() {
    scanning = true;
    hasScanned = true;
    try {
      emulators = await searchEmulators(defaultPaths);
    } catch (e) {
      uiStore.notify("扫描失败: " + String(e), "error");
    } finally {
      scanning = false;
    }
  }

  // Also allow user to pick a specific emulator folder
  async function pickEmuFolder() {
    const dir = await pickDirectory().catch(() => "");
    if (!dir) return;
    scanning = true;
    try {
      const found = await searchEmulators([dir]);
      // Merge without duplicates
      for (const f of found) {
        if (!emulators.some(e => e.id === f.id && e.install_dir === f.install_dir)) {
          emulators = [...emulators, f];
        }
      }
      if (found.length === 0) {
        uiStore.notify("该目录未检测到模拟器", "info");
      }
    } catch (e) {
      uiStore.notify("扫描失败: " + String(e), "error");
    } finally {
      scanning = false;
    }
  }

  async function selectRomDir() {
    const dir = await pickDirectory().catch(() => "");
    if (dir) romDir = dir;
  }

  async function doScanRoms() {
    if (!romDir || !selectedEmu) return;
    const profile = selectedEmu.profiles[selectedProfileIdx];
    if (!profile) return;
    scanningRoms = true;
    try {
      roms = await scanRoms(romDir, profile.image_extensions, true);
      selectedRoms = new Set(roms.map((_, i) => i));
    } catch (e) {
      uiStore.notify("扫描失败: " + String(e), "error");
    } finally {
      scanningRoms = false;
    }
  }

  function toggleAllRoms() {
    selectedRoms = selectedRoms.size === roms.length ? new Set() : new Set(roms.map((_, i) => i));
  }
  function toggleOneRom(i: number) {
    const n = new Set(selectedRoms); n.has(i) ? n.delete(i) : n.add(i); selectedRoms = n;
  }

  async function doImport() {
    if (!selectedEmu) return;
    const profile = selectedEmu.profiles[selectedProfileIdx];
    const toImport = roms.filter((_, i) => selectedRoms.has(i));
    importing = true;
    let ok = 0, fail = 0;
    for (const rom of toImport) {
      try {
        await importRomGame(
          rom.name, rom.path, selectedEmu.executable,
          profile.startup_arguments ?? `"{ImagePath}"`,
          profile.platform_ids[0] ?? "unknown",
        );
        ok++;
      } catch { fail++; }
    }
    importResult = { ok, fail };
    importing = false;
    await gameStore.load();
  }

  function close() { uiStore.currentView = "home"; }

  // Don't auto-scan — let user trigger it manually (scanning drives is slow)

  function formatSize(bytes: number): string {
    if (bytes > 1_000_000_000) return (bytes / 1_000_000_000).toFixed(1) + " GB";
    if (bytes > 1_000_000) return (bytes / 1_000_000).toFixed(1) + " MB";
    return (bytes / 1000).toFixed(1) + " KB";
  }

  // 三态统一：扫描中 loading / 未扫描引导与扫描为空 empty / 检测到模拟器 ready。
  const emuScanState = $derived<ViewState>(
    scanning ? "loading" : emulators.length > 0 ? "ready" : "empty",
  );
</script>

<PageShell as="div" width="full" scrollable={false} class="emulator-import-v2-shell" labelledBy="emulator-import-page-title" ariaLabel={i18n.t("emulator_import.title")}>
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="emulator-import-page" tabindex="-1" onkeydown={(e) => { if (e.key === "Escape") close(); }}>
    <div class="v2-grain ei-grain" aria-hidden="true"></div>

    <PageHeader
      id="emulator-import-page-title"
      class="ei-header"
      eyebrow="エミュレータ / EMULATOR"
      title={i18n.t("emulator_import.title")}
      description={i18n.t("emulator_import.subtitle")}
    >
      {#snippet actions()}
        <Button variant="ghost" class="ei-close" press={close} ariaLabel={i18n.t("emulator_import.close_aria")}>
          <Icon name="x" size={18} />
        </Button>
      {/snippet}
    </PageHeader>

    <div class="ei-content">
      <!-- Step 1: Emulators -->
      <section class="ei-section">
        <div class="section-title"><span class="step-num">01</span> {i18n.t("emulator_import.step_detect")}</div>
        <AsyncState
          state={emuScanState}
          compact
          title={scanning ? i18n.t("emulator_import.scanning") : hasScanned ? i18n.t("emulator_import.scan_empty") : i18n.t("emulator_import.step_detect")}
          description={scanning || hasScanned ? undefined : i18n.t("emulator_import.scan_prompt")}
          primaryAction={emuScanState === "empty" ? { label: i18n.t(hasScanned ? "emulator_import.rescan" : "emulator_import.scan_paths"), onSelect: doScanEmulators } : undefined}
          secondaryAction={emuScanState === "empty" ? { label: i18n.t("emulator_import.pick_folder"), onSelect: pickEmuFolder } : undefined}
          loadingRows={2}
        >
          <div class="emu-list">
            {#each emulators as emu}
              <button class="emu-row" class:active={selectedEmu?.id === emu.id && selectedEmu?.install_dir === emu.install_dir}
                onclick={() => { selectedEmu = emu; selectedProfileIdx = 0; roms = []; }}>
                <Icon name="gamepad" size={16} />
                <div class="emu-info">
                  <strong>{emu.name}</strong>
                  <span>{i18n.t("emulator_import.profiles_count", { count: emu.profiles.length })} · {emu.install_dir}</span>
                </div>
                <span class="row-action">{selectedEmu?.id === emu.id && selectedEmu?.install_dir === emu.install_dir ? i18n.t("emulator_import.selected") : i18n.t("emulator_import.select")}</span>
                <span class="badge">{emu.profiles.length}</span>
              </button>
            {/each}
          </div>
        </AsyncState>
        <div class="btn-row section-actions">
          <Button variant="secondary" press={doScanEmulators} disabled={scanning}>
            <Icon name="refresh" size={14} /> {scanning ? i18n.t("emulator_import.scanning") : i18n.t("emulator_import.rescan")}
          </Button>
          <Button variant="secondary" press={pickEmuFolder} disabled={scanning}>
            <Icon name="folder" size={14} /> {i18n.t("emulator_import.pick_folder")}
          </Button>
        </div>
      </section>

      <!-- Step 2: Choose profile + ROM dir -->
      {#if selectedEmu}
        <section class="ei-section">
          <div class="section-title"><span class="step-num">02</span> {i18n.t("emulator_import.step_profiles", { name: selectedEmu.name })}</div>
          <div class="profile-tabs">
            {#each selectedEmu.profiles as prof, i}
              <button class:active={selectedProfileIdx === i} onclick={() => { selectedProfileIdx = i; roms = []; }}>
                {prof.profile_name}
              </button>
            {/each}
          </div>
          {#if selectedEmu.profiles[selectedProfileIdx]}
            {@const p = selectedEmu.profiles[selectedProfileIdx]}
            <div class="profile-meta">
              <span>{i18n.t("emulator_import.profile_meta_platforms")}: {p.platform_ids.join(", ")}</span>
              <span>{i18n.t("emulator_import.profile_meta_extensions")}: {p.image_extensions.join(", ")}</span>
            </div>
          {/if}

          <div class="input-row">
            <input type="text" bind:value={romDir} readonly placeholder={i18n.t("emulator_import.rom_dir_placeholder")} />
            <Button variant="secondary" press={selectRomDir}>
              <Icon name="folder" size={14} /> {i18n.t("emulator_import.select")}
            </Button>
            {#if romDir}
              <Button press={doScanRoms} disabled={scanningRoms}>
                <Icon name="search" size={14} /> {scanningRoms ? i18n.t("emulator_import.scanning") : i18n.t("emulator_import.scan_roms")}
              </Button>
            {/if}
          </div>
        </section>
      {/if}

      <!-- Step 3: ROM list -->
      {#if roms.length > 0 && !importResult}
        <div class="toolbar">
          <label class="select-all">
            <input type="checkbox" checked={selectedRoms.size === roms.length} onchange={toggleAllRoms} />
            <span>{selectedRoms.size === roms.length ? i18n.t("emulator_import.deselect_all") : i18n.t("emulator_import.select_all")} · <strong class="num">{selectedRoms.size}</strong> / <strong class="num">{roms.length}</strong></span>
          </label>
        </div>
        <div class="game-list">
          {#each roms as rom, i}
            <label class="game-row">
              <input type="checkbox" checked={selectedRoms.has(i)} onchange={() => toggleOneRom(i)} />
              <span class="name">{rom.name}</span>
              <span class="row-tail">
                <span class="ext">{rom.extension}</span>
                <span class="size">{formatSize(rom.size_bytes)}</span>
              </span>
            </label>
          {/each}
        </div>
        <div class="actions">
          <Button variant="ghost" press={close}>{i18n.t("button.cancel")}</Button>
          <Button press={doImport} disabled={importing || selectedRoms.size === 0}>
            {importing ? i18n.t("emulator_import.importing") : i18n.t("emulator_import.import_count", { count: selectedRoms.size })}
          </Button>
        </div>
      {/if}

      {#if importResult}
        <div class="result">
          <p><Icon name="check" size={16} /> {i18n.t("emulator_import.import_success")}: <strong class="num">{importResult.ok}</strong></p>
          {#if importResult.fail > 0}<p><Icon name="x" size={16} /> {i18n.t("emulator_import.import_failed")}: <strong class="num">{importResult.fail}</strong></p>{/if}
        </div>
        <Button press={close}>{i18n.t("emulator_import.done")}</Button>
      {/if}
    </div>
  </div>
</PageShell>

<style>
  :global(.emulator-import-v2-shell) { height: 100%; }
  :global(.emulator-import-v2-shell .v2-page-shell__inner) { height: 100%; padding: 0; }

  .emulator-import-page {
    position: relative;
    isolation: isolate;
    height: 100%;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    background: var(--bg-void);
    color: var(--text-primary);
    outline: none;
  }

  /* Halftone grain background layer (utility class lives in tokens-v2.css). */
  .ei-grain { position: absolute; inset: 0; z-index: 0; }

  :global(.ei-header) {
    position: relative;
    z-index: 1;
    flex-shrink: 0;
    padding: 22px 24px 0;
  }
  :global(.ui-button.ei-close) {
    width: 38px;
    height: 38px;
    padding: 0;
  }

  .ei-content {
    position: relative;
    z-index: 1;
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 18px 24px 36px;
    display: flex;
    flex-direction: column;
    gap: 14px;
    scroll-behavior: smooth;
  }

  .ei-section { padding: 4px 0 14px; border-top: 1px solid var(--border); display: flex; flex-direction: column; gap: 10px; }
  .section-title { font-weight: 650; font-size: 0.95rem; color: var(--text-primary); display: flex; align-items: center; gap: 8px; }
  .step-num {
    display: inline-flex; align-items: center; justify-content: center;
    min-width: 28px; height: 22px; border-radius: 8px;
    background: var(--accent-lo); color: var(--accent);
    font-family: var(--font-mono); font-size: 0.72rem; font-weight: 700;
    font-variant-numeric: tabular-nums;
  }
  .num { font-family: var(--font-mono); font-variant-numeric: tabular-nums; }

  .emu-list { max-height: 240px; overflow-y: auto; display: flex; flex-direction: column; border: 1px solid var(--border); border-radius: 8px; background: var(--bg-deep); }
  .emu-row { display: grid; grid-template-columns: 20px minmax(0, 1fr) auto auto; align-items: center; gap: 10px; padding: 10px 14px; border: 0; border-bottom: 1px solid var(--border); border-radius: 0; background: transparent; cursor: pointer; font-size: 0.85rem; color: var(--text-secondary); transition: background 0.15s ease, color 0.15s ease, border-color 0.15s ease; text-align: left; }
  .emu-row:last-child { border-bottom: none; }
  .emu-row:hover { border-color: var(--border-hover); }
  .emu-row.active { background: var(--accent-lo); color: var(--accent); }
  .emu-info { flex: 1; display: flex; flex-direction: column; gap: 2px; }
  .emu-info strong { color: var(--text-primary); font-weight: 600; }
  .emu-info span { font-size: 0.74rem; color: var(--text-muted); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .row-action { font-size: 0.75rem; color: var(--text-muted); }
  .emu-row.active .row-action { color: var(--accent); font-weight: 650; }
  .badge { font-family: var(--font-mono); font-size: 0.72rem; font-variant-numeric: tabular-nums; padding: 2px 7px; border-radius: var(--radius-full); background: var(--accent-lo); color: var(--accent); }

  .profile-tabs { display: flex; gap: 4px; flex-wrap: wrap; }
  .profile-tabs button { padding: 6px 14px; border: 1px solid var(--border); border-radius: var(--radius-full); background: transparent; color: var(--text-muted); cursor: pointer; font-size: 0.8rem; transition: background 0.15s ease, color 0.15s ease, border-color 0.15s ease; }
  .profile-tabs button.active { background: var(--accent-lo); border-color: var(--accent); color: var(--accent); font-weight: 600; }
  .profile-meta { display: flex; flex-direction: column; gap: 2px; font-size: 0.78rem; color: var(--text-muted); }

  .input-row { display: flex; gap: 8px; align-items: center; }
  .input-row input { flex: 1; padding: 10px 12px; border-radius: 8px; background: var(--bg-void); border: 1px solid var(--border); color: var(--text-secondary); font-size: 0.82rem; }

  .toolbar { display: flex; align-items: center; gap: 12px; }
  .btn-row { display: flex; gap: 8px; flex-wrap: wrap; align-items: center; }
  .section-actions { margin-top: 2px; }
  .select-all { display: flex; align-items: center; gap: 8px; font-size: 0.85rem; color: var(--text-secondary); cursor: pointer; }
  .select-all input { accent-color: var(--accent); }

  .game-list { max-height: 300px; overflow-y: auto; border: 1px solid var(--border); border-radius: 8px; background: var(--bg-deep); display: flex; flex-direction: column; }
  .game-row { display: grid; grid-template-columns: 20px minmax(0, 1fr) auto; align-items: center; gap: 10px; padding: 10px 14px; border-bottom: 1px solid var(--border); cursor: pointer; font-size: 0.85rem; }
  .game-row:last-child { border-bottom: none; }
  .game-row:hover { background: var(--bg-hover); }
  .game-row input { accent-color: var(--accent); }
  .name { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--text-primary); }
  .row-tail { display: inline-flex; align-items: center; gap: 8px; justify-content: flex-end; }
  .ext { font-family: var(--font-mono); font-size: 0.72rem; color: var(--accent); padding: 2px 6px; background: var(--accent-lo); border-radius: var(--radius-full); }
  .size { font-family: var(--font-mono); font-size: 0.75rem; font-variant-numeric: tabular-nums; color: var(--text-muted); }

  .actions { display: flex; gap: 10px; justify-content: flex-end; }

  .result { display: flex; flex-direction: column; gap: 6px; padding: 14px; border-radius: 8px; background: rgba(34,197,94,.08); border: 1px solid rgba(34,197,94,.18); }
  .result p { display: flex; align-items: center; gap: 6px; font-size: 0.88rem; color: var(--text-secondary); }

  @media (max-width: 560px) {
    :global(.ei-header) { padding: 18px 16px 0; }
    .ei-content { padding: 16px 16px 32px; }
    .input-row { flex-wrap: wrap; }
    .input-row input { flex: 1 1 100%; }
  }

  /* ── Reduced motion ── */
  @media (prefers-reduced-motion: reduce) {
    .ei-content { scroll-behavior: auto; }
    .emu-row, .profile-tabs button { transition: none; }
  }
  :global([data-motion="reduce"]) .ei-content { scroll-behavior: auto; }
  :global([data-motion="reduce"]) .emu-row,
  :global([data-motion="reduce"]) .profile-tabs button { transition: none; }
</style>

<script lang="ts">
  import { gameStore } from "../stores/games.svelte";
  import { uiStore } from "../stores/ui.svelte";
  import { searchEmulators, scanRoms, importRomGame, pickDirectory, type ScannedEmulator, type RomFile } from "../api";
  import Icon from "./Icon.svelte";
  import Button from "./ui/Button.svelte";

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
</script>

<div class="overlay aura-page" data-aura-echo="EMULATOR" role="dialog" tabindex="-1" onkeydown={(e) => { if (e.key === "Escape") close(); }}>
  <div class="dialog aura-panel aura-bevel">
    <header class="aura-head">
      <div>
        <p class="aura-kicker">Emulator</p>
        <h2 class="aura-title"><Icon name="gamepad" size={20} /> 模拟器与 ROM 导入</h2>
        <p>扫描模拟器、选择配置集，并把 ROM 文件写入游戏库。</p>
      </div>
      <button class="close" onclick={close} aria-label="关闭"><Icon name="x" size={18} /></button>
    </header>

    <!-- Step 1: Emulators -->
    <section class="aura-section">
      <div class="section-title"><span class="step aura-num">01</span> 检测已安装的模拟器</div>
      {#if scanning}
        <div class="aura-empty aura-inset compact"><Icon name="refresh" size={14} /> 正在扫描...</div>
      {:else if !hasScanned && emulators.length === 0}
        <div class="aura-empty aura-inset">点击下方按钮扫描已安装的模拟器，或手动选择模拟器目录。</div>
        <div class="btn-row">
          <Button press={doScanEmulators} disabled={scanning}>
            <Icon name="search" size={14} /> 扫描常见路径
          </Button>
          <Button variant="secondary" press={pickEmuFolder} disabled={scanning}>
            <Icon name="folder" size={14} /> 手动选择目录
          </Button>
        </div>
      {:else if emulators.length === 0}
        <div class="aura-empty aura-inset">未检测到模拟器。点击重新扫描或手动选择模拟器目录。</div>
      {:else}
        <div class="emu-list">
          {#each emulators as emu}
            <button class="emu-row" class:active={selectedEmu?.id === emu.id && selectedEmu?.install_dir === emu.install_dir}
              onclick={() => { selectedEmu = emu; selectedProfileIdx = 0; roms = []; }}>
              <Icon name="gamepad" size={16} />
              <div class="emu-info">
                <strong>{emu.name}</strong>
                <span>{emu.profiles.length} 个配置集 · {emu.install_dir}</span>
              </div>
              <span class="row-action">{selectedEmu?.id === emu.id && selectedEmu?.install_dir === emu.install_dir ? "已选择" : "选择"}</span>
              <span class="badge aura-num">{emu.profiles.length}</span>
            </button>
          {/each}
        </div>
      {/if}
      <div class="btn-row section-actions">
        <Button variant="secondary" press={doScanEmulators} disabled={scanning}>
          <Icon name="refresh" size={14} /> {scanning ? "扫描中..." : "重新扫描"}
        </Button>
        <Button variant="secondary" press={pickEmuFolder} disabled={scanning}>
          <Icon name="folder" size={14} /> 手动选择目录
        </Button>
      </div>
    </section>

    <!-- Step 2: Choose profile + ROM dir -->
    {#if selectedEmu}
      <section class="aura-section">
        <div class="section-title"><span class="step aura-num">02</span> {selectedEmu.name} — 配置集</div>
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
            <span>平台: {p.platform_ids.join(", ")}</span>
            <span>扩展名: {p.image_extensions.join(", ")}</span>
          </div>
        {/if}

        <div class="input-row">
          <input type="text" bind:value={romDir} readonly placeholder="点击选择 ROM 文件夹" />
          <Button variant="secondary" press={selectRomDir}>
            <Icon name="folder" size={14} /> 选择
          </Button>
          {#if romDir}
            <Button press={doScanRoms} disabled={scanningRoms}>
              <Icon name="search" size={14} /> {scanningRoms ? "扫描中..." : "扫描 ROM"}
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
          <span>{selectedRoms.size === roms.length ? "取消全选" : "全选"} · <strong class="aura-num">{selectedRoms.size}</strong> / <strong class="aura-num">{roms.length}</strong></span>
        </label>
      </div>
      <div class="game-list">
        {#each roms as rom, i}
          <label class="game-row">
            <input type="checkbox" checked={selectedRoms.has(i)} onchange={() => toggleOneRom(i)} />
            <span class="name">{rom.name}</span>
            <span class="row-tail">
              <span class="ext">{rom.extension}</span>
              <span class="size aura-num">{formatSize(rom.size_bytes)}</span>
            </span>
          </label>
        {/each}
      </div>
      <div class="actions">
        <Button variant="ghost" press={close}>取消</Button>
        <Button press={doImport} disabled={importing || selectedRoms.size === 0}>
          {importing ? "导入中..." : `导入 ${selectedRoms.size} 个游戏`}
        </Button>
      </div>
    {/if}

    {#if importResult}
      <div class="result">
        <p><Icon name="check" size={16} /> 导入成功: <strong class="aura-num">{importResult.ok}</strong></p>
        {#if importResult.fail > 0}<p><Icon name="x" size={16} /> 失败: <strong class="aura-num">{importResult.fail}</strong></p>{/if}
      </div>
      <Button press={close}>完成</Button>
    {/if}
  </div>
</div>

<style>
  .overlay { position: fixed; inset: 0; z-index: 180; display: flex; align-items: center; justify-content: center; background: rgba(0,0,0,0.64); }
  .dialog {
    width: 640px; max-width: 94vw; max-height: 86vh; padding: 28px;
    display: flex; flex-direction: column; gap: 14px; overflow-y: auto;
    border-radius: 8px; box-shadow: var(--shadow-lg);
  }
  header { display: flex; justify-content: space-between; align-items: center; }
  .aura-head { align-items: flex-start; gap: 16px; }
  .aura-kicker {
    margin: 0 0 6px;
    font-family: var(--font-mono);
    font-size: 0.72rem;
    color: var(--accent);
    text-transform: uppercase;
  }
  .aura-title { margin: 0; }
  .aura-head p { margin: 6px 0 0; color: var(--text-secondary); font-size: 0.82rem; line-height: 1.5; }
  h2 { font-size: 1.15rem; font-weight: 650; display: flex; align-items: center; gap: 8px; color: var(--text-primary); }
  .close { background: none; border: none; color: var(--text-muted); cursor: pointer; padding: 4px; border-radius: var(--radius-sm); display: flex; }
  .close:hover { color: var(--text-primary); background: var(--bg-hover); }

  .aura-section { padding: 4px 0 14px; border-top: 1px solid var(--border); display: flex; flex-direction: column; gap: 10px; }
  .section-title { font-weight: 650; font-size: 0.95rem; color: var(--text-primary); display: flex; align-items: center; gap: 8px; }
  .step { display: inline-flex; align-items: center; justify-content: center; min-width: 28px; height: 22px; border-radius: 8px; background: var(--accent-lo); color: var(--accent); font-size: 0.72rem; font-weight: 700; }
  .aura-empty {
    min-height: 76px;
    display: flex; align-items: center; justify-content: center; gap: 8px;
    padding: 14px; border: 1px dashed var(--border-hover); border-radius: 8px;
    background: var(--aura-inset); color: var(--text-muted); font-size: 0.84rem; text-align: center;
  }
  .aura-empty.compact { min-height: 48px; justify-content: flex-start; }

  .emu-list { max-height: 240px; overflow-y: auto; display: flex; flex-direction: column; border: 1px solid var(--border); border-radius: 8px; background: var(--bg-deep); }
  .emu-row { display: grid; grid-template-columns: 20px minmax(0, 1fr) auto auto; align-items: center; gap: 10px; padding: 10px 14px; border: 0; border-bottom: 1px solid var(--border); border-radius: 0; background: transparent; cursor: pointer; font-size: 0.85rem; color: var(--text-secondary); transition: all 0.15s; text-align: left; }
  .emu-row:last-child { border-bottom: none; }
  .emu-row:hover { border-color: var(--border-hover); }
  .emu-row.active { background: var(--accent-lo); color: var(--accent); }
  .emu-info { flex: 1; display: flex; flex-direction: column; gap: 2px; }
  .emu-info strong { color: var(--text-primary); font-weight: 600; }
  .emu-info span { font-size: 0.74rem; color: var(--text-muted); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .row-action { font-size: 0.75rem; color: var(--text-muted); }
  .emu-row.active .row-action { color: var(--accent); font-weight: 650; }
  .badge { font-family: var(--font-mono); font-size: 0.72rem; padding: 2px 7px; border-radius: var(--radius-full); background: var(--accent-lo); color: var(--accent); }

  .profile-tabs { display: flex; gap: 4px; flex-wrap: wrap; }
  .profile-tabs button { padding: 6px 14px; border: 1px solid var(--border); border-radius: var(--radius-full); background: transparent; color: var(--text-muted); cursor: pointer; font-size: 0.8rem; transition: all 0.15s; }
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
  .size { font-family: var(--font-mono); font-size: 0.75rem; color: var(--text-muted); }

  .actions { display: flex; gap: 10px; justify-content: flex-end; }

  .result { display: flex; flex-direction: column; gap: 6px; padding: 14px; border-radius: 8px; background: rgba(34,197,94,.08); border: 1px solid rgba(34,197,94,.18); }
  .result p { display: flex; align-items: center; gap: 6px; font-size: 0.88rem; color: var(--text-secondary); }
</style>

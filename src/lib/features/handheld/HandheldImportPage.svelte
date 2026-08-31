<script lang="ts">
  // 掌机 ROM 导入向导（Android / ES-DE 式）：
  // ① 全盘访问授权 → ② 选择模拟器 → ③ 扫描 ROM 目录 → ④ 批量入库（自动匹配封面）
  import { onMount } from "svelte";
  import { uiStore } from "../../stores/ui.svelte";
  import { gameStore } from "../../stores/games.svelte";
  import { navigateTo } from "../../stores/router.svelte";
  import { fileSrc } from "../../utils";
  import {
    hasAllFilesAccess,
    requestAllFilesAccess,
    listInstalledEmulators,
    handheldRomRoots,
    discoverRomRoots,
    handheldScanRoms,
    handheldImportRoms,
    type InstalledEmulator,
    type RomRootInfo,
    type ScannedRom,
  } from "./api";
  import { platformLabel, resolveEmulatorAssignments } from "./systems";
  import Icon from "../../components/Icon.svelte";

  let granted = $state<boolean | null>(null);
  let emulators = $state<InstalledEmulator[]>([]);
  let selectedEmu = $state<InstalledEmulator | null>(null);
  let roots = $state<RomRootInfo[]>([]);
  let romDir = $state("");
  let roms = $state<ScannedRom[]>([]);
  let selected = $state<Set<number>>(new Set());
  let scanning = $state(false);
  let importing = $state(false);
  let importResult = $state<{ ok: number; fail: number } | null>(null);

  const scannedPlatforms = $derived([...new Set(roms.map((r) => r.platform))].sort());
  const selectedCount = $derived(selected.size);
  // 按平台自动分配启动模拟器（PSP→PPSSPP、NDS→DraStic…，其余 RetroArch 兜底）
  const assignment = $derived(resolveEmulatorAssignments(scannedPlatforms, emulators));

  onMount(async () => {
    await refreshPermission();
    try {
      emulators = await listInstalledEmulators();
      selectedEmu = emulators[0] ?? null;
    } catch (e) {
      uiStore.notify("模拟器探测失败: " + String(e), "error");
    }
    try {
      // 两条来源合并：插件（StorageManager 卷枚举，覆盖外置 SD）+ Rust 静态候选
      const [pluginRoots, rustRoots] = await Promise.all([
        discoverRomRoots().catch(() => [] as RomRootInfo[]),
        handheldRomRoots(),
      ]);
      const seen = new Set<string>();
      roots = [...pluginRoots, ...rustRoots].filter((r) => {
        const key = r.path.replace(/\/+$/, "").toLowerCase();
        if (seen.has(key)) return false;
        seen.add(key);
        return true;
      });
      const first = roots.find((r) => r.exists);
      if (first) romDir = first.path;
    } catch { /* 根目录探测失败不阻塞手动输入 */ }
  });

  async function refreshPermission() {
    try {
      granted = await hasAllFilesAccess();
    } catch {
      granted = null;
    }
  }

  async function grantAccess() {
    try {
      await requestAllFilesAccess();
      uiStore.notify("已跳转系统设置，授权后返回应用点「重新检查」", "info");
    } catch (e) {
      uiStore.notify("无法打开授权页: " + String(e), "error");
    }
  }

  async function doScan() {
    if (!romDir.trim()) return;
    scanning = true;
    importResult = null;
    try {
      roms = await handheldScanRoms(romDir.trim());
      selected = new Set(roms.map((_, i) => i));
      if (roms.length === 0) uiStore.notify("该目录未发现 ROM 文件", "info");
    } catch (e) {
      uiStore.notify("扫描失败: " + String(e), "error");
      roms = [];
    } finally {
      scanning = false;
    }
  }

  function toggleAll() {
    selected = selected.size === roms.length ? new Set() : new Set(roms.map((_, i) => i));
  }
  function toggleOne(i: number) {
    const next = new Set(selected);
    if (next.has(i)) next.delete(i);
    else next.add(i);
    selected = next;
  }
  function togglePlatform(platform: string) {
    const idxs = roms.map((r, i) => (r.platform === platform ? i : -1)).filter((i) => i >= 0);
    const allOn = idxs.every((i) => selected.has(i));
    const next = new Set(selected);
    for (const i of idxs) {
      if (allOn) next.delete(i);
      else next.add(i);
    }
    selected = next;
  }

  async function doImport() {
    const items = roms
      .filter((_, i) => selected.has(i))
      .map((r) => ({
        path: r.path,
        platform: r.platform,
        name: r.name,
        // 专用模拟器优先；未匹配平台用兜底选择
        emulatorPackage: assignment.assignments[r.platform] ?? selectedEmu?.packageName,
        lastPlayed: r.lastPlayed,
        cover: r.cover,
      }));
    if (items.length === 0) return;
    importing = true;
    try {
      importResult = await handheldImportRoms(selectedEmu?.packageName ?? "", items);
      await gameStore.load();
    } catch (e) {
      uiStore.notify("导入失败: " + String(e), "error");
    } finally {
      importing = false;
    }
  }

  function formatSize(bytes: number): string {
    if (bytes > 1_000_000_000) return (bytes / 1_000_000_000).toFixed(1) + " GB";
    if (bytes > 1_000_000) return (bytes / 1_000_000).toFixed(0) + " MB";
    return (bytes / 1000).toFixed(0) + " KB";
  }

  function back() {
    // Android 终版将导入完成后的落点统一回掌机首页；旧 handheld 仍只作为兼容 hash。
    navigateTo("home");
  }
</script>

<div class="import-page">
  <header class="ip-topbar">
    <button type="button" class="ip-back" onclick={back}><Icon name="arrow-left" size={16} /> 返回</button>
    <h1>导入模拟器游戏</h1>
    <span class="ip-step-hint">授权 → 模拟器 → 扫描 → 入库</span>
  </header>

  <div class="ip-content">
    <!-- ① 权限 -->
    <section class="ip-section" class:ok={granted === true}>
      <div class="ip-section-title"><span class="ip-num">01</span> 存储访问授权</div>
      {#if granted === true}
        <p class="ip-line ok-line"><Icon name="check" size={15} /> 已拥有「所有文件访问」权限，可以扫描 ROM 目录。</p>
      {:else}
        <p class="ip-line">扫描 /sdcard 下的 ROM 需要「所有文件访问」权限。授权后回到本页重新检查。</p>
        <div class="ip-actions">
          <button type="button" class="ip-cta" onclick={grantAccess}>去授权</button>
          <button type="button" class="ip-cta secondary" onclick={refreshPermission}>重新检查</button>
        </div>
      {/if}
    </section>

    <!-- ② 模拟器 -->
    <section class="ip-section" class:dim={granted !== true}>
      <div class="ip-section-title"><span class="ip-num">02</span> 启动用模拟器</div>
      {#if emulators.length === 0}
        <p class="ip-line warn-line">未探测到已安装的常见模拟器（RetroArch / PPSSPP / DraStic / DuckStation…）。仍可导入游戏，但启动前请先安装模拟器。</p>
      {:else}
        <p class="ip-line">已探测到 {emulators.length} 个模拟器；入库时按平台自动分配（PSP→PPSSPP、NDS→DraStic、PS1→DuckStation…），未匹配的平台走下方选择的兜底模拟器。</p>
        <div class="ip-emu-list">
          {#each emulators as emu}
            <button
              type="button"
              class="ip-emu"
              class:active={selectedEmu?.packageName === emu.packageName}
              onclick={() => (selectedEmu = emu)}
            >
              <strong>{emu.label}{selectedEmu?.packageName === emu.packageName ? "（兜底）" : ""}</strong>
              <span>{emu.packageName}</span>
            </button>
          {/each}
        </div>
      {/if}
    </section>

    <!-- ③ 扫描 -->
    <section class="ip-section" class:dim={granted !== true}>
      <div class="ip-section-title"><span class="ip-num">03</span> 扫描 ROM 目录</div>
      <div class="ip-roots">
        {#each roots as root}
          <button
            type="button"
            class="ip-root"
            class:active={romDir === root.path}
            class:missing={!root.exists}
            disabled={!root.exists}
            onclick={() => (romDir = root.path)}
          >{root.label}</button>
        {/each}
      </div>
      <div class="ip-dir-row">
        <input type="text" bind:value={romDir} placeholder="/storage/emulated/0/ROMs" />
        <button type="button" class="ip-cta" onclick={doScan} disabled={scanning || granted !== true}>
          {scanning ? "扫描中…" : "扫描"}
        </button>
      </div>

      {#if roms.length > 0}
        <div class="ip-platform-chips">
          {#each scannedPlatforms as platform}
            {@const idxs = roms.map((r, i) => (r.platform === platform ? i : -1)).filter((i) => i >= 0)}
            <button type="button" class="ip-chip" onclick={() => togglePlatform(platform)}>
              {platformLabel(platform)} · {idxs.filter((i) => selected.has(i)).length}/{idxs.length}
            </button>
          {/each}
          <button type="button" class="ip-chip all" onclick={toggleAll}>
            {selected.size === roms.length ? "全不选" : "全选"} · {selectedCount}/{roms.length}
          </button>
        </div>
        <div class="ip-rom-list">
          {#each roms as rom, i}
            <label class="ip-rom">
              <input type="checkbox" checked={selected.has(i)} onchange={() => toggleOne(i)} />
              <span class="ip-rom-cover">
                {#if rom.cover}<img src={fileSrc(rom.cover)} alt="" loading="lazy" />{/if}
              </span>
              <span class="ip-rom-info">
                <strong>{rom.name}</strong>
                <span>{platformLabel(rom.platform)} · {formatSize(rom.sizeBytes)}</span>
              </span>
            </label>
          {/each}
        </div>
      {/if}
    </section>

    <!-- ④ 导入 -->
    {#if roms.length > 0 || importResult}
      <section class="ip-section">
        <div class="ip-section-title"><span class="ip-num">04</span> 入库</div>
        {#if importResult}
          <p class="ip-line ok-line"><Icon name="check" size={15} /> 成功导入 {importResult.ok} 款{#if importResult.fail > 0}，失败 {importResult.fail} 款{/if}。</p>
          <div class="ip-actions">
            <button type="button" class="ip-cta" onclick={back}>进入掌机模式</button>
          </div>
        {:else}
          <p class="ip-line">将选中的 {selectedCount} 款 ROM 入库。封面与游戏名优先取 ES-DE 数据（downloaded_media / gamelist）。</p>
          <div class="ip-assign">
            {#each scannedPlatforms as platform}
              {@const emuLabel = assignment.labels[platform]}
              <span class="ip-assign-chip" class:warn={!emuLabel}>
                {platformLabel(platform)} → {emuLabel ?? "未匹配模拟器"}
              </span>
            {/each}
          </div>
          <div class="ip-actions">
            <button type="button" class="ip-cta" onclick={doImport} disabled={importing || selectedCount === 0 || assignment.missing.length > 0}>
              {importing ? "导入中…" : `导入 ${selectedCount} 款游戏`}
            </button>
          </div>
        {/if}
      </section>
    {/if}
  </div>
</div>

<style>
  .import-page {
    height: 100%;
    display: flex; flex-direction: column;
    background: var(--bg-void); color: var(--text-primary);
    font-family: var(--font-ui);
    overflow: hidden;
  }
  .ip-topbar {
    display: flex; align-items: center; gap: 16px;
    padding: 14px 22px;
    border-bottom: 1px solid var(--border);
  }
  .ip-topbar h1 { margin: 0; font-family: var(--font-display); font-size: 1.1rem; font-weight: 750; }
  .ip-back {
    display: inline-flex; align-items: center; gap: 6px;
    padding: 8px 12px; border-radius: var(--radius-md);
    border: 1px solid var(--border);
    background: transparent; color: var(--text-secondary);
    font: 600 .8rem/1 var(--font-ui); cursor: pointer;
    transition: border-color .18s ease, color .18s ease;
  }
  .ip-back:hover { border-color: var(--border-hover); color: var(--text-primary); }
  .ip-step-hint { margin-left: auto; font-size: .72rem; color: var(--text-muted); letter-spacing: .06em; }

  .ip-content {
    flex: 1; min-height: 0; overflow-y: auto;
    padding: 18px 22px 40px;
    display: flex; flex-direction: column; gap: 14px;
  }
  .ip-section {
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--glass-bg);
    backdrop-filter: var(--glass-blur);
    -webkit-backdrop-filter: var(--glass-blur);
    box-shadow: var(--glass-highlight);
    padding: 14px 16px;
    display: flex; flex-direction: column; gap: 10px;
  }
  .ip-section.ok { border-color: color-mix(in srgb, var(--color-success) 38%, transparent); }
  .ip-section.dim { opacity: .45; pointer-events: none; }
  .ip-section-title { font-weight: 700; font-size: .92rem; display: flex; align-items: center; gap: 8px; }
  .ip-num {
    min-width: 26px; height: 20px; border-radius: 7px;
    display: inline-flex; align-items: center; justify-content: center;
    background: var(--accent-lo); color: var(--accent);
    font: 700 .68rem/1 var(--font-mono);
  }
  .ip-line { margin: 0; font-size: .82rem; color: var(--text-secondary); line-height: 1.6; }
  .ok-line { color: var(--color-success); display: flex; align-items: center; gap: 6px; }
  .warn-line { color: var(--color-warning); }
  .ip-actions { display: flex; gap: 10px; }

  .ip-cta {
    padding: 11px 20px; border: 0; border-radius: var(--radius-md);
    background: var(--accent); color: #fff;
    font: 700 .82rem/1 var(--font-ui); cursor: pointer;
    transition: background .18s ease;
  }
  .ip-cta:hover { background: var(--accent-hi); }
  .ip-cta:disabled { opacity: .5; }
  .ip-cta.secondary { background: var(--bg-elev); border: 1px solid var(--border); color: var(--text-primary); }
  .ip-cta.secondary:hover { border-color: var(--border-hover); background: var(--bg-hover); }

  .ip-emu-list { display: grid; grid-template-columns: repeat(auto-fill, minmax(210px, 1fr)); gap: 8px; }
  .ip-emu {
    display: flex; flex-direction: column; gap: 3px; align-items: flex-start;
    padding: 10px 13px; border-radius: var(--radius-md);
    border: 1px solid var(--border);
    background: transparent; color: var(--text-secondary); cursor: pointer; text-align: left;
    transition: border-color .18s ease, background .18s ease;
  }
  .ip-emu:hover { border-color: var(--border-hover); }
  .ip-emu strong { font-size: .85rem; color: var(--text-primary); }
  .ip-emu span { font-size: .68rem; color: var(--text-muted); font-family: var(--font-mono); }
  .ip-emu.active { border-color: var(--accent-ring); background: var(--accent-lo); }

  .ip-roots { display: flex; gap: 8px; flex-wrap: wrap; }
  .ip-root {
    padding: 8px 14px; border-radius: var(--radius-full);
    border: 1px solid var(--border);
    background: transparent; color: var(--text-secondary);
    font: 600 .78rem/1 var(--font-ui); cursor: pointer;
    transition: border-color .18s ease, color .18s ease, background .18s ease;
  }
  .ip-root:hover { border-color: var(--border-hover); color: var(--text-primary); }
  .ip-root.active { border-color: var(--accent-ring); color: var(--accent-hi); background: var(--accent-lo); }
  .ip-root.missing { opacity: .35; }

  .ip-dir-row { display: flex; gap: 8px; }
  .ip-dir-row input {
    flex: 1; padding: 11px 14px; border-radius: var(--radius-md);
    border: 1px solid var(--border);
    background: var(--bg-deep); color: var(--text-primary);
    font: .82rem/1 var(--font-mono);
    outline: none;
    transition: border-color .18s ease, box-shadow .18s ease;
  }
  .ip-dir-row input:focus-visible { border-color: var(--accent); box-shadow: 0 0 0 3px var(--accent-ring); }

  .ip-platform-chips { display: flex; gap: 8px; flex-wrap: wrap; }
  .ip-assign { display: flex; gap: 8px; flex-wrap: wrap; }
  .ip-assign-chip {
    padding: 6px 12px; border-radius: var(--radius-full);
    border: 1px solid color-mix(in srgb, var(--color-success) 38%, transparent);
    background: color-mix(in srgb, var(--color-success) 8%, transparent);
    color: color-mix(in srgb, var(--color-success) 72%, white);
    font: 600 .72rem/1 var(--font-ui);
  }
  .ip-assign-chip.warn {
    border-color: color-mix(in srgb, var(--color-warning) 42%, transparent);
    background: color-mix(in srgb, var(--color-warning) 8%, transparent);
    color: var(--color-warning);
  }
  .ip-chip {
    padding: 7px 13px; border-radius: var(--radius-full);
    border: 1px solid var(--border);
    background: transparent; color: var(--text-secondary);
    font: 600 .74rem/1 var(--font-ui); cursor: pointer;
    transition: border-color .18s ease, color .18s ease;
  }
  .ip-chip:hover { border-color: var(--border-hover); color: var(--text-primary); }
  .ip-chip.all { border-color: var(--border-hover); color: var(--text-primary); }

  .ip-rom-list {
    max-height: 320px; overflow-y: auto;
    border: 1px solid var(--border); border-radius: var(--radius-md);
    display: flex; flex-direction: column;
    background: color-mix(in srgb, var(--bg-deep) 60%, transparent);
  }
  .ip-rom {
    display: grid; grid-template-columns: 22px 44px minmax(0, 1fr);
    align-items: center; gap: 10px;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
    cursor: pointer;
  }
  .ip-rom:hover { background: var(--bg-hover); }
  .ip-rom:last-child { border-bottom: 0; }
  .ip-rom input { accent-color: var(--accent); }
  .ip-rom-cover {
    width: 44px; height: 56px; border-radius: 6px; overflow: hidden;
    background: var(--bg-card);
  }
  .ip-rom-cover img { width: 100%; height: 100%; object-fit: cover; }
  .ip-rom-info { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
  .ip-rom-info strong { font-size: .84rem; color: var(--text-primary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .ip-rom-info span { font-size: .7rem; color: var(--text-muted); font-family: var(--font-mono); }
</style>

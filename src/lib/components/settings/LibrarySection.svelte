<script lang="ts">
  import { settingsStore } from "../../stores/settings.svelte";
  import { uiStore } from "../../stores/ui.svelte";
  import { i18n } from "../../stores/i18n.svelte";
  import { platformStore } from "../../platform";
  import Button from "../ui/Button.svelte";
  import Card from "../ui/Card.svelte";
  import Icon from "../Icon.svelte";
  import "./settings-shared.css";
</script>

<span class="section-anchor" id="settings-library" aria-hidden="true"></span>
<Card class="s-section" padding="lg" ariaLabel="settings-library">
  <div class="s-head">
    <h2 class="s-title"><Icon name="folder" size={17} className="s-title-ic" /> {platformStore.capabilities.localGameScan ? i18n.t("settings.section_library") : i18n.t("settings.section_library_mobile")}<span class="s-title-sub">ライブラリ / LIBRARY</span></h2>
  </div>

  {#if platformStore.capabilities.localGameScan}
  <div class="s-info" style="padding-bottom: 10px;">
    <span class="s-label">扫描目录</span>
    <span class="s-desc">萌游会监视这些目录中的游戏</span>
  </div>
  {#if settingsStore.settings.watch_dirs.length > 0}
    <div class="dir-list">
      {#each settingsStore.settings.watch_dirs as dir}
        <Card class="dir-item" padding="sm" hoverable>
          <span class="dir-path">{dir}</span>
          <button class="dir-remove" onclick={() => settingsStore.removeWatchDir(dir)} title="移除" type="button">
            <Icon name="x" size={14} />
          </button>
        </Card>
      {/each}
    </div>
  {:else}
    <div class="s-empty">
      <Icon name="folder" size={28} />
      <span>尚未添加扫描目录</span>
    </div>
  {/if}
  <div style="padding: 12px 0 4px;">
    <Button variant="secondary" press={() => settingsStore.addWatchDir()}>添加目录</Button>
  </div>

  <div class="s-divider"></div>
  <div class="ops-list">
    {#if platformStore.capabilities.steamIntegration}
    <div class="ops-item">
      <div class="ops-info">
        <Icon name="steam" size={18} className="ops-icon" />
        <div>
          <span class="s-label">Steam / Epic 导入</span>
          <span class="s-desc">扫描本机已安装游戏，或通过登录同步完整库</span>
        </div>
      </div>
      <Button variant="primary" size="sm" press={() => uiStore.currentView = "steam-import"}>打开</Button>
    </div>
    {/if}
  </div>
  {:else}
    <div class="s-empty" role="status">
      <Icon name="smartphone" size={28} />
      <span>手机版只管理游戏资料与同步数据，不扫描本机目录，也不提供 Steam / Epic 导入。</span>
    </div>
  {/if}
</Card>

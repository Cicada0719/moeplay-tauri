<script lang="ts">
  import { settingsStore } from "../../stores/settings.svelte";
  import { uiStore } from "../../stores/ui.svelte";
  import { i18n } from "../../stores/i18n.svelte";
  import { platformStore } from "../../platform";
  import Card from "../ui/Card.svelte";
  import Switch from "../ui/Switch.svelte";
  import Input from "../ui/Input.svelte";
  import Icon from "../Icon.svelte";
  import "./settings-shared.css";

  const scrapeSources = [
    { key: "vndb_enabled" as const, label: "VNDB", description: "" },
    { key: "bangumi_enabled" as const, label: "Bangumi", description: "" },
    { key: "dlsite_enabled" as const, label: "DLsite", description: "" },
    { key: "getchu_enabled" as const, label: "Getchu", description: "日本 Galgame 商店资料，响应较慢，默认关闭" },
    { key: "kungal_enabled" as const, label: "Kungal", description: "" },
    { key: "steam_enabled" as const, label: "Steam", description: "" },
    { key: "pcgw_enabled" as const, label: "PCGW", description: "" },
    { key: "erogamescape_enabled" as const, label: "批评空间", description: "" },
    { key: "ymgal_enabled" as const, label: "月幕 Ymgal", description: "" },
    { key: "touchgal_enabled" as const, label: "TouchGAL", description: "" },
  ];
  const visibleScrapeSources = $derived(
    scrapeSources.filter((source) => source.key !== "steam_enabled" || platformStore.capabilities.steamIntegration),
  );

  function isSourceEnabled(key: string): boolean {
    return !!(settingsStore.settings as any)[key];
  }

  async function save() {
    await settingsStore.save(settingsStore.settings);
  }

  async function toggleScrapeSetting(key: string) {
    await settingsStore.save({ ...settingsStore.settings, [key]: !isSourceEnabled(key) });
    uiStore.notify("设置已保存", "success");
  }

</script>

<span class="section-anchor" id="settings-scrape" aria-hidden="true"></span>
<Card class="s-section" padding="lg" ariaLabel="settings-scrape">
  <div class="s-head">
    <h2 class="s-title"><Icon name="layers" size={17} className="s-title-ic" /> {i18n.t("settings.section_scrape")}<span class="s-title-sub">スクレイプ / SCRAPE</span></h2>
  </div>
  <div class="src-grid">
    {#each visibleScrapeSources as src}
      <div class="src-item">
        <div class="src-info">
          <span class="src-name">{src.label}</span>
          {#if src.description}<span class="src-desc">{src.description}</span>{/if}
        </div>
        <Switch checked={isSourceEnabled(src.key)} onchange={() => toggleScrapeSetting(src.key)} />
      </div>
    {/each}
  </div>
  <div class="s-divider"></div>
  <div class="src-item">
    <div class="src-info">
      <span class="src-name">自动刮削</span>
      <span class="src-desc">{platformStore.isAndroid ? "添加或编辑资料时自动搜索并填充元数据" : "添加游戏时自动搜索并填充元数据"}</span>
    </div>
    <Switch checked={settingsStore.settings.auto_scrape} onchange={() => toggleScrapeSetting("auto_scrape")} />
  </div>
  <div class="s-divider"></div>
  <div class="s-row" style="flex-direction: column; align-items: stretch; gap: 6px;">
    <div class="s-info">
      <span class="s-label">HTTP 代理</span>
      <span class="s-desc">刮削数据源时使用的代理地址，留空则使用系统代理</span>
    </div>
    <Input
      bind:value={settingsStore.settings.scraper_proxy}
      onblur={save}
      placeholder="如 http://127.0.0.1:7890（留空 = 系统代理）"
    />
  </div>
</Card>

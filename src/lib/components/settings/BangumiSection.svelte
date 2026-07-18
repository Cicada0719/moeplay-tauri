<script lang="ts">
  import { animeStore } from "../../stores/anime.svelte";
  import { uiStore } from "../../stores/ui.svelte";
  import { i18n } from "../../stores/i18n.svelte";
  import Button from "../ui/Button.svelte";
  import Card from "../ui/Card.svelte";
  import Input from "../ui/Input.svelte";
  import SegmentControl from "../ui/SegmentControl.svelte";
  import Icon from "../Icon.svelte";
  import "./settings-shared.css";

  let bangumiTokenInput = $state("");
  let bangumiConnecting = $state(false);
  let bangumiConnectMsg = $state("");
  let bangumiSyncing = $state(false);

  async function handleBangumiConnect() {
    if (!bangumiTokenInput.trim()) {
      bangumiConnectMsg = "请输入 Access Token";
      return;
    }
    bangumiConnecting = true;
    bangumiConnectMsg = "";
    try {
      const username = await animeStore.setBangumiToken(bangumiTokenInput.trim());
      bangumiConnectMsg = `已连接: ${username}`;
      uiStore.notify(`Bangumi 已连接: ${username}`, "success");
    } catch (e) {
      bangumiConnectMsg = `连接失败: ${e}`;
    } finally {
      bangumiConnecting = false;
    }
  }

  async function handleBangumiDisconnect() {
    animeStore.disconnectBangumi();
    bangumiTokenInput = "";
    bangumiConnectMsg = "";
    uiStore.notify("已断开 Bangumi 连接", "success");
  }

  async function handleBangumiSync() {
    bangumiSyncing = true;
    try {
      await animeStore.loadBangumiCollection();
      await animeStore.syncBangumiToLocal();
      uiStore.notify(animeStore.bangumiSyncProgress || "同步完成", "success");
    } catch (e) {
      uiStore.notify(`同步失败: ${e}`, "error");
    } finally {
      bangumiSyncing = false;
    }
  }
</script>

<span class="section-anchor" id="settings-bangumi" aria-hidden="true"></span>
<Card class="s-section" padding="lg" ariaLabel="settings-bangumi">
  <div class="s-head">
    <h2 class="s-title"><Icon name="heart" size={17} className="s-title-ic" /> {i18n.t("settings.section_bangumi")}<span class="s-title-sub">同期 / BANGUMI</span></h2>
  </div>
  <p class="s-note">连接你的 Bangumi 账号，自动同步番剧收藏状态</p>

  <div class="ops-list">
    <div class="ops-item" style="flex-direction: column; align-items: stretch; gap: 10px;">
      <div class="s-info">
        <span class="s-label">Access Token</span>
        <span class="s-desc">从 Bangumi 个人设置 → 开发者 → Access Token 获取</span>
      </div>
      <div style="display: flex; gap: 8px; align-items: center;">
        <div style="flex: 1;">
          <Input
            type="password"
            bind:value={bangumiTokenInput}
            placeholder="粘贴你的 Bangumi Access Token"
          />
        </div>
        {#if animeStore.bangumiConnected}
          <Button variant="ghost" size="sm" press={handleBangumiDisconnect}>断开</Button>
        {:else}
          <Button variant="primary" size="sm" press={handleBangumiConnect} disabled={bangumiConnecting}>
            {bangumiConnecting ? "连接中..." : "连接"}
          </Button>
        {/if}
      </div>
      {#if bangumiConnectMsg}
        <div class="ops-msg">{bangumiConnectMsg}</div>
      {/if}
      {#if animeStore.bangumiConnected}
        <div class="ops-msg" style="color: #58d68d;">
          ✓ 已连接: {animeStore.bangumiUsername}
        </div>
      {/if}
    </div>
  </div>

  {#if animeStore.bangumiConnected}
    <div class="s-divider"></div>

    <div class="ops-list">
      <div class="ops-item">
        <div class="ops-info">
          <Icon name="refresh" size={18} className="ops-icon" />
          <div>
            <span class="s-label">同步收藏</span>
            <span class="s-desc">拉取 Bangumi 收藏并与本地合并</span>
          </div>
        </div>
        <Button variant="primary" size="sm" press={handleBangumiSync} disabled={bangumiSyncing}>
          {bangumiSyncing ? "同步中..." : "同步"}
        </Button>
      </div>

      <div class="ops-item">
        <div class="ops-info">
          <span class="s-label" style="margin-left: 26px;">同步优先级</span>
          <span class="s-desc">冲突时以哪方为准</span>
        </div>
        <SegmentControl
          options={[
            { value: "0", label: "本地优先" },
            { value: "1", label: "Bangumi 优先" },
          ]}
          value={String(animeStore.bangumiSyncPriority)}
          onChange={(v) => animeStore.bangumiSyncPriority = parseInt(v)}
          size="sm"
        />
      </div>
    </div>

    {#if animeStore.bangumiSyncProgress}
      <div class="ops-msg">{animeStore.bangumiSyncProgress}</div>
    {/if}
    {#if animeStore.bangumiSyncError}
      <div class="ops-msg" style="color: #e8557f;">{animeStore.bangumiSyncError}</div>
    {/if}
  {/if}
</Card>

<script lang="ts">
  // 设置页「内置源规则」区块（spec task-02 Step 7.5）：
  // 展示规则包版本/来源/更新时间/数量，「检查更新」「立即健康检查」按钮。
  import { onMount } from "svelte";
  import { uiStore } from "../../stores/ui.svelte";
  import {
    checkAndUpdateRules,
    getRulesMeta,
    importKazumiRules,
    probeHealth,
    type RulesMetaInfo,
  } from "../../api/rules";
  import Card from "../ui/Card.svelte";
  import Button from "../ui/Button.svelte";
  import Icon from "../Icon.svelte";
  import "./settings-shared.css";

  let meta: RulesMetaInfo | null = $state(null);
    let checking = $state(false);
    let probing = $state(false);
    let syncingKazumi = $state(false);

    async function onSyncKazumi() {
      syncingKazumi = true;
      try {
        const result = await importKazumiRules(true);
        const total = result.imported + result.unchanged;
        uiStore.notify(
          `Kazumi 规则库同步完成：${total} 个源可用（新增 ${result.imported}，跳过 ${result.unchanged}）`,
          result.syncFailed > 0 ? "error" : "success",
        );
      } catch (e) {
        uiStore.notify(`Kazumi 同步失败：${String(e)}`, "error");
      } finally {
        syncingKazumi = false;
      }
    }

  onMount(() => {
    void getRulesMeta()
      .then((m) => {
        meta = m;
      })
      .catch(() => {
        meta = null;
      });
  });

  function formatTs(ts: number | null | undefined): string {
    if (ts == null) return "—";
    try {
      return new Date(ts * 1000).toLocaleString();
    } catch {
      return String(ts);
    }
  }

  async function onCheckUpdate() {
    checking = true;
    try {
      const result = await checkAndUpdateRules(true);
      if (result.status === "updated") {
        uiStore.notify(`规则已更新至 v${result.toVersion}（${result.updatedRules} 条）`, "success");
      } else if (result.status === "alreadyLatest") {
        uiStore.notify("已是最新", "info");
      } else {
        // FallbackCached：更新失败，已继续使用本地规则（FR-04 用户无感知中断）
        uiStore.notify("更新失败，已继续使用本地规则", "info");
      }
      meta = await getRulesMeta().catch(() => meta);
    } catch (e) {
      uiStore.notify(`检查更新失败：${String(e)}`, "error");
    } finally {
      checking = false;
    }
  }

  async function onProbeHealth() {
    probing = true;
    try {
      const results = await probeHealth();
      const ok = results.filter((r) => r.ok).length;
      uiStore.notify(`健康检查完成：${ok}/${results.length} 个源可用`, "success");
    } catch (e) {
      uiStore.notify(`健康检查失败：${String(e)}`, "error");
    } finally {
      probing = false;
    }
  }
</script>

<span class="section-anchor" id="settings-rules" aria-hidden="true"></span>
<Card class="s-section" padding="lg" ariaLabel="settings-rules">
  <div class="s-head">
    <h2 class="s-title"><Icon name="layers" size={17} className="s-title-ic" /> 内置源规则<span class="s-title-sub">RULES / ルール</span></h2>
  </div>

  <p class="s-note">内置源规则包可带签名校验静默热更新；健康检查探测各源搜索可用性，异常源在源列表中沉底显示。</p>

  <div class="src-grid">
    <div class="src-item">
      <div class="src-info">
        <span class="src-name">规则包版本</span>
        <span class="src-desc" data-testid="rules-meta-version">{meta?.packageVersion ?? "…"}</span>
      </div>
    </div>
    <div class="src-item">
      <div class="src-info">
        <span class="src-name">来源</span>
        <span class="src-desc" data-testid="rules-meta-source">
          {meta ? (meta.source === "remoteCache" ? "远端缓存" : "内置") : "…"}
        </span>
      </div>
    </div>
    <div class="src-item">
      <div class="src-info">
        <span class="src-name">更新时间</span>
        <span class="src-desc" data-testid="rules-meta-updated">{meta ? formatTs(meta.updatedAt) : "…"}</span>
      </div>
    </div>
    <div class="src-item">
      <div class="src-info">
        <span class="src-name">规则数量</span>
        <span class="src-desc" data-testid="rules-meta-count">{meta?.ruleCount ?? "…"} 条</span>
      </div>
    </div>
  </div>

  <div class="s-divider"></div>

  <div class="src-item">
    <div class="src-info">
      <span class="src-name">检查更新</span>
      <span class="src-desc">从远端规则仓库拉取最新规则包；网络/签名失败自动继续使用本地规则</span>
    </div>
    <Button
      size="sm"
      variant="secondary"
      press={onCheckUpdate}
      loading={checking}
    >
      检查更新
    </Button>
  </div>

  <div class="src-item">
    <div class="src-info">
      <span class="src-name">立即健康检查</span>
      <span class="src-desc">对全部内置源执行一次搜索探测，刷新健康状态并落盘</span>
    </div>
    <Button
      size="sm"
      variant="secondary"
      press={onProbeHealth}
      loading={probing}
    >
      立即健康检查
    </Button>
  </div>

  <div class="src-item">
    <div class="src-info">
      <span class="src-name">同步 Kazumi 规则库</span>
      <span class="src-desc">从 kazumi 官方规则仓库（KazumiRules）拉取最新源；kazumi 更新源后点这里即可跟进，无需等应用更新</span>
    </div>
    <Button
      size="sm"
      variant="secondary"
      press={onSyncKazumi}
      loading={syncingKazumi}
    >
      同步 Kazumi 源
    </Button>
  </div>
</Card>

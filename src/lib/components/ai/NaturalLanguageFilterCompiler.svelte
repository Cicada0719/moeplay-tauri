<script lang="ts">
  import Button from "../ui/Button.svelte";
  import Input from "../ui/Input.svelte";
  import Icon from "../Icon.svelte";
  import { GenerationGuard, getAiClient, isAbortError, isAiUnavailableError, type AiClient } from "../../features/ai";
  import type { NaturalLanguageFilterDsl, StructuredFilterFallback } from "../../features/ai/types";
  import { buildStructuredFallback, serializeFilterDsl, validateFilterDslResult } from "../../features/ai/validation";

  let {
    client = getAiClient(),
    onApply,
  }: {
    client?: AiClient;
    onApply?: (dsl: NaturalLanguageFilterDsl) => void;
  } = $props();

  let mode = $state<"natural" | "structured">("natural");
  let query = $state("");
  let phase = $state<"idle" | "loading" | "valid" | "invalid" | "offline" | "error" | "cancelled">("idle");
  let validatedDsl = $state<NaturalLanguageFilterDsl | null>(null);
  let errors = $state<string[]>([]);
  let fallback = $state<StructuredFilterFallback>({
    keyword: "",
    tag: "",
    maxHours: "",
    unplayedOnly: false,
    contentRating: "any",
    sort: "affinity",
  });
  const guard = new GenerationGuard();

  async function compileNaturalLanguage() {
    if (!query.trim()) {
      errors = ["请先输入希望查找的作品条件。"];
      phase = "invalid";
      return;
    }
    const request = guard.begin();
    phase = "loading";
    validatedDsl = null;
    errors = [];
    try {
      const result = await client.compileFilter({ query: query.trim(), kind: "game", generation: request.generation }, request.signal);
      if (!guard.isCurrent(request.generation) || result.generation !== request.generation) return;
      const validation = validateFilterDslResult(result.dsl, "game");
      if (!validation.ok) {
        errors = validation.errors;
        phase = "invalid";
        return;
      }
      validatedDsl = validation.value;
      phase = "valid";
    } catch (error) {
      if (!guard.isCurrent(request.generation) || isAbortError(error)) return;
      errors = [isAiUnavailableError(error) ? "AI 不可用，已保留本地结构化筛选入口。" : "筛选编译失败，未生成可应用的 DSL。"];
      phase = isAiUnavailableError(error) ? "offline" : "error";
    }
  }

  function cancelCompile() {
    guard.cancel();
    validatedDsl = null;
    errors = ["已取消本次编译；迟到结果不会更新界面。"];
    phase = "cancelled";
  }

  function compileFallback() {
    guard.cancel();
    const candidate = buildStructuredFallback(fallback);
    const validation = validateFilterDslResult(candidate, "game");
    if (!validation.ok) {
      validatedDsl = null;
      errors = validation.errors;
      phase = "invalid";
      return;
    }
    validatedDsl = validation.value;
    errors = [];
    phase = "valid";
  }

  function applyDsl() {
    if (phase !== "valid" || !validatedDsl) return;
    onApply?.(validatedDsl);
  }
</script>

<section class="compiler" aria-labelledby="filter-compiler-title">
  <div class="compiler-head">
    <div>
      <p class="eyebrow">LOCAL DSL COMPILER</p>
      <h2 id="filter-compiler-title">自然语言筛选</h2>
      <p>AI 只能生成白名单 DSL；校验失败、离线或取消时都不能应用结果。</p>
    </div>
    <div class="mode-switch" aria-label="筛选输入方式">
      <button class:active={mode === "natural"} type="button" onclick={() => mode = "natural"}>自然语言</button>
      <button class:active={mode === "structured"} type="button" onclick={() => mode = "structured"}>本地筛选</button>
    </div>
  </div>

  {#if mode === "natural"}
    <div class="natural-box">
      <label for="natural-filter-query">描述你想找的作品</label>
      <textarea id="natural-filter-query" bind:value={query} placeholder="例如：找最近没玩过、全年龄、轻松、十小时以内的作品"></textarea>
      <div class="action-row">
        <span>不会执行 SQL、脚本或系统命令</span>
        {#if phase === "loading"}
          <Button variant="ghost" size="sm" press={cancelCompile}>取消编译</Button>
        {:else}
          <Button variant="primary" size="sm" press={compileNaturalLanguage}>编译为本地 DSL</Button>
        {/if}
      </div>
    </div>
  {:else}
    <div class="fallback-grid">
      <label>标题关键词<Input bind:value={fallback.keyword} placeholder="可选" ariaLabel="标题关键词" /></label>
      <label>标签<Input bind:value={fallback.tag} placeholder="例如：治愈" ariaLabel="标签" /></label>
      <label>最长时长<Input bind:value={fallback.maxHours} type="number" min="0" max="10000" placeholder="小时" ariaLabel="最长时长" /></label>
      <label>内容分级
        <select bind:value={fallback.contentRating} aria-label="内容分级">
          <option value="any">不限</option><option value="all_ages">全年龄</option><option value="teen">青少年</option><option value="mature">成熟</option><option value="adult">成人</option><option value="unknown">未知</option>
        </select>
      </label>
      <label>排序
        <select bind:value={fallback.sort} aria-label="排序">
          <option value="affinity">偏好优先</option><option value="recent">最近添加</option><option value="title">标题</option>
        </select>
      </label>
      <label class="check-row"><input type="checkbox" bind:checked={fallback.unplayedOnly} />仅未玩过</label>
      <div class="fallback-action"><Button variant="secondary" size="sm" press={compileFallback}>生成本地 DSL</Button></div>
    </div>
  {/if}

  {#if phase === "loading"}
    <div class="result-state loading" aria-live="polite"><span class="spinner"></span>正在生成并校验筛选 DSL…</div>
  {:else if errors.length}
    <div class="result-state" class:offline={phase === "offline"} class:error={phase === "invalid" || phase === "error"} role="alert">
      <Icon name={phase === "offline" ? "cloudOff" : phase === "cancelled" ? "square" : "x"} size={15} />
      <div>{#each errors as error}<p>{error}</p>{/each}</div>
      {#if phase === "offline"}<Button variant="quiet" size="sm" press={() => mode = "structured"}>使用本地筛选</Button>{/if}
    </div>
  {/if}

  {#if validatedDsl}
    <div class="dsl-preview">
      <div class="dsl-head">
        <div><strong>已验证的本地 DSL</strong><span>{validatedDsl.explanation}</span></div>
        <span class="validated"><Icon name="check" size={13} />已通过白名单校验</span>
      </div>
      <pre aria-label="已验证的筛选 DSL">{serializeFilterDsl(validatedDsl)}</pre>
      <div class="apply-row">
        <p>只有当前显示的已验证版本会被应用。</p>
        <Button variant="primary" size="sm" press={applyDsl}>应用筛选</Button>
      </div>
    </div>
  {/if}
</section>

<style>
  .compiler { display: grid; gap: 14px; }
  .compiler-head { display: flex; align-items: flex-start; justify-content: space-between; gap: 16px; }
  .eyebrow { margin: 0 0 5px; color: var(--accent); font-family: var(--font-mono); font-size: 10px; font-weight: 700; letter-spacing: .13em; }
  h2, p { margin: 0; }
  h2 { color: var(--text-primary); font-size: 1.12rem; }
  .compiler-head p:not(.eyebrow) { margin-top: 5px; color: var(--text-secondary); font-size: 12px; line-height: 1.5; }
  .mode-switch { padding: 3px; border: 1px solid var(--border); border-radius: 7px; display: flex; background: var(--bg-deep); }
  .mode-switch button { min-height: 28px; padding: 0 10px; border: 0; border-radius: 5px; background: transparent; color: var(--text-muted); font: 600 11px var(--font-ui); cursor: pointer; }
  .mode-switch button.active { background: var(--bg-elev); color: var(--text-primary); }
  .natural-box, .fallback-grid, .dsl-preview { padding: 14px; border: 1px solid var(--border); border-radius: 8px; background: var(--bg-card); }
  .natural-box { display: grid; gap: 9px; }
  label { display: grid; gap: 7px; color: var(--text-secondary); font-size: 11px; font-weight: 600; }
  textarea, select { width: 100%; border: 1px solid var(--border); border-radius: 7px; background: var(--bg-elev); color: var(--text-primary); font-family: var(--font-ui); font-size: 12px; outline: none; }
  textarea { min-height: 92px; padding: 11px 12px; resize: vertical; line-height: 1.55; }
  select { min-height: 38px; padding: 0 10px; }
  textarea:focus-visible, select:focus-visible { border-color: var(--accent); box-shadow: 0 0 0 2px var(--accent-ring); }
  .action-row, .apply-row { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
  .action-row > span, .apply-row p { color: var(--text-muted); font-size: 10px; }
  .fallback-grid { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 12px; }
  .check-row { min-height: 38px; display: flex; align-items: center; gap: 8px; align-self: end; }
  .check-row input { accent-color: var(--accent); }
  .fallback-action { display: flex; align-items: end; justify-content: flex-end; }
  .result-state { min-height: 42px; padding: 10px 12px; border: 1px solid var(--border); border-radius: 8px; display: flex; align-items: center; gap: 9px; background: var(--bg-elev); color: var(--text-secondary); font-size: 11px; }
  .result-state > div { flex: 1; display: grid; gap: 3px; }
  .result-state.error { border-color: color-mix(in srgb, var(--color-error, #f87171) 35%, var(--border)); color: var(--color-error, #f87171); }
  .result-state.offline { border-color: color-mix(in srgb, var(--color-warning, #fbbf24) 35%, var(--border)); color: var(--color-warning, #fbbf24); }
  .spinner { width: 14px; height: 14px; border: 2px solid currentColor; border-right-color: transparent; border-radius: 50%; animation: spin .8s linear infinite; }
  .dsl-preview { display: grid; gap: 12px; }
  .dsl-head { display: flex; justify-content: space-between; gap: 14px; }
  .dsl-head > div { min-width: 0; display: grid; gap: 4px; }
  .dsl-head strong { color: var(--text-primary); font-size: 12px; }
  .dsl-head div span { color: var(--text-secondary); font-size: 10px; }
  .validated { height: 24px; padding: 0 8px; border: 1px solid color-mix(in srgb, var(--color-success, #4ade80) 35%, transparent); border-radius: 999px; display: inline-flex; align-items: center; gap: 5px; color: var(--color-success, #4ade80); font-size: 9px; font-weight: 700; white-space: nowrap; }
  pre { max-height: 250px; margin: 0; padding: 12px; overflow: auto; border: 1px solid var(--border); border-radius: 7px; background: var(--bg-deep); color: var(--text-secondary); font: 10px/1.55 var(--font-mono); white-space: pre-wrap; }
  @keyframes spin { to { transform: rotate(360deg); } }
  @media (max-width: 780px) { .fallback-grid { grid-template-columns: repeat(2, minmax(0, 1fr)); } }
  @media (max-width: 560px) { .compiler-head, .action-row, .dsl-head, .apply-row { align-items: stretch; flex-direction: column; } .mode-switch { width: 100%; } .mode-switch button { flex: 1; } .fallback-grid { grid-template-columns: 1fr; } }
  @media (prefers-reduced-motion: reduce) { .spinner { animation: none; } }
</style>

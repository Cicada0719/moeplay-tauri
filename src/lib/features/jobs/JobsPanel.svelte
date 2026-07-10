<script lang="ts">
  import { onMount } from "svelte";
  import { createJobsStore, type JobsStore, type Job } from ".";

  export let store: JobsStore = createJobsStore();
  let jobs: Job[] = [];
  let loading = false;
  let error: string | null = null;

  const unsubscribe = store.subscribe((snapshot) => {
    jobs = snapshot.jobs;
    loading = snapshot.loading;
    error = snapshot.error;
  });

  onMount(() => {
    void store.load();
    return unsubscribe;
  });

  const statusLabel: Record<Job["status"], string> = {
    queued: "排队中",
    running: "运行中",
    paused: "已暂停",
    succeeded: "已完成",
    failed: "失败",
    cancelled: "已取消",
  };
</script>

<section class="jobs" aria-label="后台任务">
  <header class="jobs__header">
    <h2>后台任务</h2>
    <button type="button" onclick={() => store.clearFinished()} disabled={loading}>清理已完成</button>
  </header>
  {#if error}<p role="alert">{error}</p>{/if}
  {#if loading && jobs.length === 0}<p>加载中…</p>{/if}
  {#if jobs.length === 0 && !loading}<p>暂无后台任务</p>{/if}
  <ul>
    {#each jobs as job (job.id)}
      <li data-status={job.status}>
        <div>
          <strong>{job.title}</strong>
          <span>{job.recovered ? "已恢复 · " : ""}{statusLabel[job.status]}</span>
        </div>
        <progress max="1" value={job.progress}>{Math.round(job.progress * 100)}%</progress>
        {#if job.message}<p>{job.message}</p>{/if}
        {#if job.kind === "download" && job.status === "running"}
          <button type="button" onclick={() => store.pause(job.id)}>暂停</button>
        {/if}
        {#if job.kind === "download" && job.status === "paused" && job.resumable}
          <button type="button" onclick={() => store.resume(job.id)}>继续</button>
        {/if}
        {#if job.kind === "download" && (job.status === "failed" || job.status === "paused") && job.retryable}
          <button type="button" onclick={() => store.retry(job.id)}>重试</button>
        {/if}
        {#if job.status === "queued" || job.status === "running" || job.status === "paused"}
          <button type="button" onclick={() => store.cancel(job.id)}>取消</button>
        {/if}
      </li>
    {/each}
  </ul>
</section>

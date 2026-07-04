<script lang="ts">
  import { get } from 'svelte/store';
  import { dismiss, retry, type Op, type OpState } from '$lib/stores/ops';
  import { settings } from '$lib/stores/settings';
  import { copyText } from '$lib/clipboard';
  import { X, ChevronDown, ChevronRight, RotateCw, ShieldAlert, Copy, Check } from '@lucide/svelte';

  let { op, queueAhead = 0 }: { op: Op; queueAhead?: number } = $props();

  let copied = $state(false);
  async function copyOutput() {
    if (await copyText(op.lines.join('\n'))) {
      copied = true;
      setTimeout(() => (copied = false), 1200);
    }
  }

  let expanded = $state(get(settings).showOutput);
  let body: HTMLDivElement | null = $state(null);

  const labels: Record<OpState, string> = {
    queued: 'Queued',
    running: 'Working',
    done: 'Done',
    error: 'Failed'
  };

  let lastLine = $derived(op.lines.filter((l) => l.trim()).at(-1) ?? '');

  // Heuristic: did this fail because it needed administrator rights?
  const ADMIN_RE =
    /(access is denied|administrator|requires elevation|elevat|run as admin|0x80070005)/i;
  let needsAdmin = $derived(op.state === 'error' && op.lines.some((l) => ADMIN_RE.test(l)));

  // Always reveal output when something fails.
  $effect(() => {
    if (op.state === 'error') expanded = true;
  });

  // Keep output scrolled to the newest line.
  $effect(() => {
    void op.lines.length;
    if (expanded && body) body.scrollTop = body.scrollHeight;
  });
</script>

<div class="toast card">
  <div class="head">
    <span class="title">{op.title}</span>
    <span class="state {op.state}">{labels[op.state]}</span>
    <div class="spacer"></div>
    <button class="icon-btn" onclick={() => dismiss(op.id)} aria-label="Dismiss">
      <X size={15} />
    </button>
  </div>

  {#if op.detail}
    <div class="detail">{op.detail}</div>
  {/if}

  <div class="progress {op.state}">
    <div class="bar"></div>
  </div>

  <div class="status">
    <span class="status-text {op.state}">
      {#if op.state === 'queued'}
        {queueAhead > 0 ? `Waiting · ${queueAhead} ahead` : 'Waiting…'}
      {:else if op.state === 'running'}
        {lastLine || 'Working…'}
      {:else if op.state === 'done'}
        Completed successfully
      {:else}
        Something went wrong
      {/if}
    </span>
    {#if op.lines.length > 0}
      <button class="toggle" onclick={copyOutput} title="Copy output">
        {#if copied}<Check size={14} />{:else}<Copy size={14} />{/if}
        {copied ? 'Copied' : 'Copy'}
      </button>
    {/if}
    {#if op.lines.length > 0 || op.state === 'running'}
      <button class="toggle" onclick={() => (expanded = !expanded)}>
        {#if expanded}<ChevronDown size={14} />{:else}<ChevronRight size={14} />{/if}
        {expanded ? 'Hide' : 'Show'} output
      </button>
    {/if}
  </div>

  {#if op.state === 'error'}
    <div class="err-actions">
      {#if needsAdmin}
        <span class="admin">
          <ShieldAlert size={13} /> Needs administrator — relaunch Acy as admin, then retry.
        </span>
      {/if}
      <div class="spacer"></div>
      <button class="retry" onclick={() => retry(op.id)}>
        <RotateCw size={13} /> Retry
      </button>
    </div>
  {/if}

  {#if expanded}
    <div class="body mono" bind:this={body}>
      {#each op.lines as line, i (i)}
        <div class="line">{line || ' '}</div>
      {/each}
      {#if op.lines.length === 0}
        <div class="line muted">{op.state === 'queued' ? 'Queued…' : 'Starting…'}</div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .toast {
    display: flex;
    flex-direction: column;
    overflow: hidden;
    box-shadow: var(--shadow);
  }
  .head {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
  }
  .title {
    font-weight: 600;
    font-size: 0.9rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .state {
    font-size: 0.7rem;
    font-family: var(--font-mono);
    padding: 1px 7px;
    border-radius: var(--radius-sm);
    border: 1px solid currentColor;
    flex-shrink: 0;
  }
  .state.queued {
    color: var(--text-muted);
  }
  .state.running {
    color: var(--warning);
  }
  .state.done {
    color: var(--success);
  }
  .state.error {
    color: var(--danger);
  }
  .spacer {
    flex: 1;
  }
  .detail {
    padding: 0 12px 9px;
    font-size: 0.76rem;
    color: var(--text-muted);
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .icon-btn {
    display: inline-flex;
    padding: 4px;
    border: none;
    background: transparent;
    color: var(--text-muted);
    border-radius: var(--radius-sm);
    line-height: 0;
  }
  .icon-btn:hover {
    background: var(--surface-hover);
    color: var(--text);
  }

  .progress {
    position: relative;
    height: 4px;
    background: var(--surface-hover);
    overflow: hidden;
  }
  .progress .bar {
    position: absolute;
    height: 100%;
    border-radius: var(--radius-pill);
  }
  .progress.running .bar {
    width: 35%;
    background: var(--accent-fill);
    animation: indeterminate 1.1s ease-in-out infinite;
  }
  .progress.done .bar {
    width: 100%;
    background: var(--success);
  }
  .progress.error .bar {
    width: 100%;
    background: var(--danger);
  }
  @keyframes indeterminate {
    0% {
      left: -35%;
    }
    100% {
      left: 100%;
    }
  }

  .status {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 9px 12px;
  }
  .status-text {
    flex: 1;
    min-width: 0;
    font-family: var(--font-mono);
    font-size: 0.74rem;
    color: var(--text-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .status-text.done {
    color: var(--success);
  }
  .status-text.error {
    color: var(--danger);
  }
  .toggle {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    background: none;
    border: none;
    color: var(--text-muted);
    font-size: 0.76rem;
    padding: 4px 6px;
    border-radius: var(--radius-sm);
    flex-shrink: 0;
  }
  .toggle:hover {
    color: var(--text);
    background: var(--surface-hover);
  }

  .body {
    max-height: 30vh;
    overflow-y: auto;
    padding: 10px 12px;
    background: var(--surface-2);
    border-top: 1px solid var(--border);
    font-size: 0.75rem;
    line-height: 1.45;
  }
  .line {
    white-space: pre-wrap;
    word-break: break-word;
  }

  .err-actions {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 12px 10px;
  }
  .err-actions .spacer {
    flex: 1;
  }
  .admin {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 0.74rem;
    color: var(--warning);
    min-width: 0;
  }
  .retry {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    flex-shrink: 0;
    border: 1px solid var(--border-strong);
    background: var(--surface);
    color: var(--text);
    padding: 5px 12px;
    border-radius: var(--radius-sm);
    font-size: 0.8rem;
    font-weight: 500;
  }
  .retry:hover {
    background: var(--surface-hover);
  }
</style>

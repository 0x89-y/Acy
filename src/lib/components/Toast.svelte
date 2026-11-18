<script lang="ts">
  import { get } from 'svelte/store';
  import { dismiss, retry, type Op, type OpState } from '$lib/stores/ops';
  import { settings } from '$lib/stores/settings';
  import { copyText } from '$lib/clipboard';
  import { X, ChevronDown, ChevronRight, RotateCw, ShieldAlert, Copy, Check, CircleAlert } from '@lucide/svelte';

  /**
   * One operation, docked in the queue tray's details panel: a card like the
   * Installed/Activity rows. The tray bar shows overall progress, so there's no
   * per-operation progress bar here.
   */
  let { op }: { op: Op } = $props();

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

  const ADMIN_RE =
    /(access is denied|administrator|requires elevation|elevat|run as admin|0x80070005)/i;
  let needsAdmin = $derived(op.state === 'error' && op.lines.some((l) => ADMIN_RE.test(l)));

  $effect(() => {
    if (op.state === 'error') expanded = true;
  });

  $effect(() => {
    void op.lines.length;
    if (expanded && body) body.scrollTop = body.scrollHeight;
  });
</script>

<div class="op {op.state}">
  <div class="row">
    <span class="ic" aria-hidden="true">
      {#if op.state === 'running' || op.state === 'queued'}
        <span class="spinner"></span>
      {:else if op.state === 'done'}
        <Check size={17} />
      {:else}
        <CircleAlert size={17} />
      {/if}
    </span>
    <span class="text">
      <span class="title-line">
        <span class="title">{op.title}</span>
        <span class="pill {op.state}">{labels[op.state]}</span>
      </span>
      <span class="status" class:mono={op.state === 'running' && !!lastLine}>
        {#if op.state === 'queued'}
          Waiting…
        {:else if op.state === 'running'}
          {lastLine || 'Starting…'}
        {:else if op.state === 'done'}
          Completed successfully
        {:else}
          Something went wrong{op.detail ? ` · ${op.detail}` : ''}
        {/if}
      </span>
    </span>
    {#if op.lines.length > 0}
      <button class="btn btn-ghost small" onclick={copyOutput}>
        {#if copied}<Check size={14} />{:else}<Copy size={14} />{/if}
        {copied ? 'Copied' : 'Copy'}
      </button>
    {/if}
    {#if op.lines.length > 0 || op.state === 'running'}
      <button class="btn btn-ghost small" onclick={() => (expanded = !expanded)} aria-expanded={expanded}>
        {#if expanded}<ChevronDown size={14} />{:else}<ChevronRight size={14} />{/if}
        {expanded ? 'Hide output' : 'Show output'}
      </button>
    {/if}
    <button class="icon-button dismiss" onclick={() => dismiss(op.id)} aria-label={`Dismiss ${op.title}`}>
      <X size={15} />
    </button>
  </div>

  {#if op.state === 'error'}
    <div class="err-actions">
      {#if needsAdmin}
        <span class="admin">
          <ShieldAlert size={14} aria-hidden="true" /> Needs administrator rights: relaunch Acy as admin, then retry.
        </span>
      {/if}
      <div class="spacer"></div>
      <button class="btn small" onclick={() => retry(op.id)}>
        <RotateCw size={14} /> Retry
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
  .op {
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface-2);
  }
  .row {
    display: flex;
    align-items: center;
    gap: 12px;
    min-height: 56px;
    padding: 8px 8px 8px 12px;
  }
  .ic {
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    border-radius: var(--radius-sm);
    background: color-mix(in srgb, var(--accent) 14%, transparent);
    color: color-mix(in srgb, var(--accent) 70%, var(--text));
  }
  .done .ic {
    background: color-mix(in srgb, var(--success) 14%, transparent);
    color: var(--success);
  }
  .error .ic {
    background: color-mix(in srgb, var(--danger) 14%, transparent);
    color: var(--danger);
  }
  .spinner {
    width: 14px;
    height: 14px;
    border: 2px solid color-mix(in srgb, currentColor 30%, transparent);
    border-top-color: currentColor;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  .text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .title-line {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .title {
    font-weight: 600;
    font-size: var(--fs-base);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .pill {
    flex-shrink: 0;
    padding: 0 8px;
    border-radius: var(--radius-pill);
    font-size: var(--fs-2xs);
    font-weight: 600;
    line-height: 1.7;
  }
  .pill.queued {
    color: var(--text-muted);
    background: color-mix(in srgb, var(--text) 8%, transparent);
  }
  .pill.running {
    color: color-mix(in srgb, var(--accent) 70%, var(--text));
    background: color-mix(in srgb, var(--accent) 16%, transparent);
  }
  .pill.done {
    color: var(--success);
    background: color-mix(in srgb, var(--success) 14%, transparent);
  }
  .pill.error {
    color: var(--danger);
    background: color-mix(in srgb, var(--danger) 14%, transparent);
  }
  .status {
    font-size: var(--fs-sm);
    color: var(--text-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .status.mono {
    font-family: var(--font-mono);
    font-size: var(--fs-xs);
  }
  .error .status {
    color: var(--danger);
  }
  .small {
    flex-shrink: 0;
    padding: 5px 10px;
    font-size: var(--fs-sm);
  }
  .btn-ghost.small {
    color: var(--text-muted);
  }
  .btn-ghost.small:hover {
    color: var(--text);
  }
  .dismiss {
    flex-shrink: 0;
  }
  .err-actions {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 12px 10px 56px;
  }
  .spacer {
    flex: 1;
  }
  .admin {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    font-size: var(--fs-sm);
    color: var(--warning);
  }
  .body {
    max-height: 30vh;
    overflow-y: auto;
    margin: 0 12px 12px 56px;
    padding: 10px 12px;
    border-radius: var(--radius-sm);
    background: color-mix(in srgb, var(--text) 5%, transparent);
    font-size: var(--fs-xs);
    line-height: 1.45;
  }
  .line {
    white-space: pre-wrap;
    word-break: break-word;
  }
</style>

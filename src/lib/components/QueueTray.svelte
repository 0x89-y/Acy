<script lang="ts">
  import { ChevronUp, ChevronDown, X, Check, CircleAlert } from '@lucide/svelte';
  import { ops, dismissAll } from '$lib/stores/ops';
  import { installQueue, batch, installQueued, clearQueue, unqueueApp } from '$lib/stores/queue';
  import Toast from './Toast.svelte';
  import QueuedOps from './QueuedOps.svelte';

  const CHIP_MAX = 3;

  let expanded = $state(false);
  let height = $state(0);

  let running = $derived($ops.find((o) => o.state === 'running') ?? null);
  let waiting = $derived($ops.filter((o) => o.state === 'queued'));
  let failed = $derived($ops.filter((o) => o.state === 'error'));
  let done = $derived($ops.filter((o) => o.state === 'done'));
  let active = $derived(!!running || waiting.length > 0);
  let showOps = $derived($ops.length > 0);
  let visible = $derived(showOps || $installQueue.length > 0);

  // Lets the notice toasts sit above the tray.
  $effect(() => {
    document.documentElement.style.setProperty('--tray-h', `${visible ? height : 0}px`);
  });
  $effect(() => {
    if (!showOps) expanded = false;
  });

  // With the details open, the cards already name each operation, so the bar
  // sums them up instead of repeating one ("1 working · 2 waiting").
  let summaryText = $derived(
    [
      running ? '1 working' : '',
      waiting.length ? `${waiting.length} waiting` : '',
      failed.length ? `${failed.length} failed` : '',
      done.length ? `${done.length} done` : ''
    ]
      .filter(Boolean)
      .join(' · ')
  );

  let statusText = $derived.by(() => {
    if (expanded && summaryText) return summaryText;
    if ($batch) return `Installing ${$batch.name}`;
    if (running) return running.title;
    if (waiting.length > 0) return `${waiting.length} waiting`;
    if (failed.length > 0) return `${failed.length} ${failed.length === 1 ? 'operation' : 'operations'} failed`;
    if (done.length === 1) return `${done[0].title} - done`;
    return `${done.length} operations done`;
  });
  let progressText = $derived(
    $batch
      ? `${$batch.current} of ${$batch.total}`
      : running && waiting.length > 0
        ? `${waiting.length} more waiting`
        : ''
  );
</script>

{#if visible}
  <div class="tray" role="region" aria-label="Install queue" bind:clientHeight={height}>
    {#if expanded && showOps}
      <div class="details" id="tray-details">
        {#each $ops as op (op.id)}
          {#if op.state !== 'queued'}
            <Toast {op} />
          {:else if op.id === waiting[0]?.id}
            <QueuedOps ops={waiting} />
          {/if}
        {/each}
      </div>
    {/if}

    {#if showOps}
      <div class="bar">
        {#if active}
          <div class="progress" aria-hidden="true"><div class="indeterminate"></div></div>
          <span class="spinner" aria-hidden="true"></span>
        {:else if failed.length > 0}
          <CircleAlert size={16} class="ic-err" aria-hidden="true" />
        {:else}
          <Check size={16} class="ic-ok" aria-hidden="true" />
        {/if}
        <span class="status truncate">{statusText}</span>
        {#if progressText}<span class="muted mono progress-text">{progressText}</span>{/if}
        <div class="spacer"></div>
        <button
          class="btn btn-ghost small"
          onclick={() => (expanded = !expanded)}
          aria-expanded={expanded}
          aria-controls="tray-details"
        >
          {#if expanded}<ChevronDown size={15} />{:else}<ChevronUp size={15} />{/if}
          {expanded ? 'Hide details' : 'Details'}
        </button>
        {#if !active}
          <button class="icon-button" onclick={dismissAll} aria-label="Dismiss finished operations" title="Dismiss">
            <X size={16} />
          </button>
        {/if}
      </div>
    {/if}

    {#if $installQueue.length > 0}
      <div class="bar queue">
        <span class="status">
          Install queue · {$installQueue.length} {$installQueue.length === 1 ? 'app' : 'apps'}
        </span>
        <div class="chips">
          {#each $installQueue.slice(0, CHIP_MAX) as app (app.key)}
            <span class="chip">
              <span class="truncate">{app.name}</span>
              <button class="chip-x" onclick={() => unqueueApp(app.key)} aria-label={`Remove ${app.name} from the queue`}>
                <X size={12} />
              </button>
            </span>
          {/each}
          {#if $installQueue.length > CHIP_MAX}
            <span class="more muted">+{$installQueue.length - CHIP_MAX} more</span>
          {/if}
        </div>
        <div class="spacer"></div>
        <button class="btn btn-ghost small" onclick={clearQueue}>Clear</button>
        <button class="btn btn-accent small" onclick={installQueued} disabled={!!$batch}>
          {$batch ? 'Wait for current installs' : `Install ${$installQueue.length}`}
        </button>
      </div>
    {/if}
  </div>
{/if}

<style>
  .tray {
    flex-shrink: 0;
    position: relative;
    border-top: 1px solid var(--border);
    background: var(--surface-2);
  }
  .details {
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-height: 45vh;
    overflow-y: auto;
    padding: 10px 14px;
    border-bottom: 1px solid var(--border);
  }
  .bar {
    position: relative;
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 48px;
    padding: 6px 14px;
  }
  .bar + .bar {
    border-top: 1px solid var(--border);
  }
  .status {
    font-size: var(--fs-base);
    font-weight: 600;
  }
  .progress-text {
    flex-shrink: 0;
    font-size: var(--fs-xs);
  }
  .truncate {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .spacer {
    flex: 1;
  }
  .small {
    padding: 6px 12px;
    font-size: var(--fs-md);
  }
  .bar :global(.ic-ok) {
    flex-shrink: 0;
    color: var(--success);
  }
  .bar :global(.ic-err) {
    flex-shrink: 0;
    color: var(--danger);
  }
  .spinner {
    width: 14px;
    height: 14px;
    flex-shrink: 0;
    border: 2px solid var(--border-strong);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  .progress {
    position: absolute;
    top: -1px;
    left: 0;
    right: 0;
    height: 2px;
    overflow: hidden;
  }
  .indeterminate {
    position: absolute;
    top: 0;
    bottom: 0;
    width: 30%;
    background: var(--accent);
    animation: slide 1.2s ease-in-out infinite;
  }
  @keyframes slide {
    from {
      left: -30%;
    }
    to {
      left: 100%;
    }
  }
  .chips {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    overflow: hidden;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    max-width: 160px;
    padding: 2px 4px 2px 10px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-pill);
    background: var(--surface);
    font-size: var(--fs-sm);
  }
  .chip-x {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    width: 22px;
    height: 22px;
    padding: 0;
    border: none;
    border-radius: 50%;
    background: transparent;
    color: var(--text-muted);
  }
  .chip-x:hover {
    background: var(--surface-hover);
    color: var(--text);
  }
  .more {
    flex-shrink: 0;
    font-size: var(--fs-sm);
  }
</style>

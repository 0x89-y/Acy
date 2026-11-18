<script lang="ts">
  import { X } from '@lucide/svelte';
  import { ops, notices, dismissNotice } from '$lib/stores/ops';

  // Spoken status per operation. Kept to the state (not the streaming output)
  // so screen readers hear "Installing X: done" once, not every log line.
  const SPOKEN = { queued: 'queued', running: 'in progress', done: 'done', error: 'failed' };
</script>

<!-- Always mounted: a live region only announces changes made after it exists. -->
<div class="sr-only" aria-live="polite">
  {#each $ops as op (op.id)}<span>{op.title}: {SPOKEN[op.state]}. </span>{/each}
  {#each $notices as n (n.id)}<span>{n.message} </span>{/each}
</div>

<!-- Short confirmations ("Added X to your list"). Operations live in the queue tray. -->
{#if $notices.length > 0}
  <div class="stack">
    {#each $notices as n (n.id)}
      <div class="notice card" class:warn={n.kind === 'warn'} class:err={n.kind === 'error'}>
        <span class="dot"></span>
        <span class="msg">{n.message}</span>
        <button class="x" onclick={() => dismissNotice(n.id)} aria-label="Dismiss">
          <X size={14} />
        </button>
      </div>
    {/each}
  </div>
{/if}

<style>
  .stack {
    position: fixed;
    right: 18px;
    bottom: calc(var(--tray-h, 0px) + 16px);
    z-index: 40;
    display: flex;
    flex-direction: column;
    gap: 10px;
    width: min(380px, calc(100vw - 36px));
  }
  .notice {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 11px 12px;
    box-shadow: var(--shadow);
  }
  .notice .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
    background: var(--success);
  }
  .notice.warn .dot {
    background: var(--warning);
  }
  .notice.err .dot {
    background: var(--danger);
  }
  .notice .msg {
    flex: 1;
    min-width: 0;
    font-size: var(--fs-md);
    font-weight: 500;
  }
  .notice .x {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    min-width: 24px;
    min-height: 24px;
    padding: 4px;
    border: none;
    background: transparent;
    color: var(--text-muted);
    border-radius: var(--radius-sm);
    line-height: 0;
  }
  .notice .x:hover {
    background: var(--surface-hover);
    color: var(--text);
  }
</style>

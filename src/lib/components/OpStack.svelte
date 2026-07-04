<script lang="ts">
  import { X } from '@lucide/svelte';
  import { ops, notices, dismissAll, dismissNotice } from '$lib/stores/ops';
  import { setSettingsTab } from '$lib/stores/settings';
  import Toast from './Toast.svelte';

  let dismissable = $derived(
    $ops.filter((o) => o.state === 'done' || o.state === 'error').length
  );
</script>

{#if $ops.length > 0 || $notices.length > 0}
  <div class="stack">
    {#if $ops.length > 0}
      <div class="stack-head">
        <a class="history" href="/settings" onclick={() => setSettingsTab('about')}>Activity</a>
        {#if dismissable > 1}
          <button class="clear-all" onclick={dismissAll} title="Dismiss finished notifications">
            Clear all
          </button>
        {/if}
      </div>
    {/if}
    {#each $notices as n (n.id)}
      <div class="notice card" class:warn={n.kind === 'warn'} class:err={n.kind === 'error'}>
        <span class="dot"></span>
        <span class="msg">{n.message}</span>
        <button class="x" onclick={() => dismissNotice(n.id)} aria-label="Dismiss">
          <X size={14} />
        </button>
      </div>
    {/each}
    {#each $ops as op, i (op.id)}
      {@const queueAhead = op.state === 'queued'
        ? $ops.slice(0, i).filter((item) => item.state === 'queued' || item.state === 'running').length
        : 0}
      <Toast {op} {queueAhead} />
    {/each}
  </div>
{/if}

<style>
  .stack {
    position: fixed;
    right: 18px;
    bottom: 18px;
    z-index: 40;
    display: flex;
    flex-direction: column;
    gap: 10px;
    width: min(420px, calc(100vw - 36px));
    max-height: calc(100vh - 36px);
    overflow-y: auto;
  }
  .stack-head {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 8px;
  }
  .history,
  .clear-all {
    border: 1px solid var(--border-strong);
    background: var(--surface);
    color: var(--text);
    padding: 4px 12px;
    border-radius: var(--radius-sm);
    font-size: 0.78rem;
    font-weight: 500;
    box-shadow: var(--shadow);
  }
  .history {
    text-decoration: none;
  }
  .clear-all {
    font-family: inherit;
  }
  .history:hover,
  .clear-all:hover {
    background: var(--surface-hover);
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
    font-size: 0.88rem;
    font-weight: 500;
  }
  .notice .x {
    display: inline-flex;
    flex-shrink: 0;
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

<script lang="ts">
  import { ops, dismissAll } from '$lib/stores/ops';
  import Toast from './Toast.svelte';

  let dismissable = $derived(
    $ops.filter((o) => o.state === 'done' || o.state === 'error').length
  );
</script>

{#if $ops.length > 0}
  <div class="stack">
    {#if dismissable > 1}
      <div class="stack-head">
        <button class="clear-all" onclick={dismissAll} title="Dismiss finished toasts">
          Clear all
        </button>
      </div>
    {/if}
    {#each $ops as op (op.id)}
      <Toast {op} />
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
    justify-content: flex-end;
  }
  .clear-all {
    border: 1px solid var(--border-strong);
    background: var(--surface);
    color: var(--text);
    padding: 4px 12px;
    border-radius: var(--radius-pill);
    font-size: 0.78rem;
    font-weight: 500;
    box-shadow: var(--shadow);
  }
  .clear-all:hover {
    background: var(--surface-hover);
  }
</style>

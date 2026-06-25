<script lang="ts">
  import { ops } from '$lib/stores/ops';
  import Toast from './Toast.svelte';
</script>

{#if $ops.length > 0}
  <div class="stack">
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
</style>

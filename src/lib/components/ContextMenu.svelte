<script lang="ts">
  import { contextMenu, closeContextMenu, type CtxItem } from '$lib/stores/contextMenu';

  function select(it: CtxItem) {
    if (it.disabled) return;
    closeContextMenu();
    it.onSelect();
  }

  function onWindowContextMenu(e: MouseEvent) {
    closeContextMenu();
    const t = e.target as HTMLElement | null;
    if (!t?.closest('input, textarea, [contenteditable="true"]')) {
      e.preventDefault();
    }
  }
</script>

<svelte:window
  onclick={closeContextMenu}
  onscroll={closeContextMenu}
  oncontextmenu={onWindowContextMenu}
  onkeydown={(e) => e.key === 'Escape' && closeContextMenu()}
/>

{#if $contextMenu}
  <div class="ctx card" style="left:{$contextMenu.x}px; top:{$contextMenu.y}px">
    {#each $contextMenu.items as it (it.label)}
      <button
        class="ctx-item"
        class:danger={it.danger}
        disabled={it.disabled}
        onclick={() => select(it)}
      >
        {it.label}
      </button>
    {/each}
  </div>
{/if}

<style>
  .ctx {
    position: fixed;
    z-index: 60;
    min-width: 168px;
    padding: 4px;
    display: flex;
    flex-direction: column;
    box-shadow: var(--shadow);
  }
  .ctx-item {
    text-align: left;
    border: none;
    background: transparent;
    color: var(--text);
    padding: 8px 10px;
    border-radius: var(--radius-sm);
    font-size: 0.86rem;
  }
  .ctx-item:hover:not(:disabled) {
    background: var(--surface-hover);
  }
  .ctx-item.danger {
    color: var(--danger);
  }
  .ctx-item:disabled {
    opacity: 0.5;
  }
</style>

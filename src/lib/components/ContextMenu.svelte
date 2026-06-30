<script lang="ts">
  import { tick } from 'svelte';
  import { contextMenu, closeContextMenu, type CtxItem } from '$lib/stores/contextMenu';

  let menu = $state<HTMLDivElement | null>(null);

  $effect(() => {
    if (!$contextMenu) return;
    void tick().then(() => menu?.querySelector<HTMLButtonElement>('button:not(:disabled)')?.focus());
  });

  function select(it: CtxItem) {
    if (it.disabled) return;
    closeContextMenu();
    it.onSelect();
  }

  // Suppress the default WebView right-click menu (Back / Reload / Save as / …)
  // everywhere except inside editable fields, which keep their copy/paste menu.
  // Card/row right-clicks stop propagation, so this only fires for empty areas.
  function onWindowContextMenu(e: MouseEvent) {
    closeContextMenu();
    const t = e.target as HTMLElement | null;
    if (!t?.closest('input, textarea, [contenteditable="true"]')) {
      e.preventDefault();
    }
  }

  function onKeydown(e: KeyboardEvent) {
    if (!$contextMenu) return;
    if (e.key === 'Escape' || e.key === 'Tab') {
      closeContextMenu();
      return;
    }
    if (!['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(e.key) || !menu) return;
    e.preventDefault();
    const items = Array.from(menu.querySelectorAll<HTMLButtonElement>('button:not(:disabled)'));
    if (items.length === 0) return;
    const current = items.indexOf(document.activeElement as HTMLButtonElement);
    const next = e.key === 'Home'
      ? 0
      : e.key === 'End'
        ? items.length - 1
        : e.key === 'ArrowDown'
          ? (current + 1 + items.length) % items.length
          : (current - 1 + items.length) % items.length;
    items[next]?.focus();
  }
</script>

<svelte:window
  onclick={closeContextMenu}
  onscroll={closeContextMenu}
  oncontextmenu={onWindowContextMenu}
  onkeydown={onKeydown}
/>

{#if $contextMenu}
  <div
    class="ctx card"
    style="left:{$contextMenu.x}px; top:{$contextMenu.y}px"
    role="menu"
    aria-label="App actions"
    bind:this={menu}
  >
    {#each $contextMenu.items as it (it.label)}
      <button
        class="ctx-item"
        class:danger={it.danger}
        disabled={it.disabled}
        onclick={() => select(it)}
        role="menuitem"
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

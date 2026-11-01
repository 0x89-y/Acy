<script module lang="ts">
  import { writable } from 'svelte/store';
  export const shortcutsOpen = writable(false);
  export function toggleShortcuts() {
    shortcutsOpen.update((v) => !v);
  }
</script>

<script lang="ts">
  import { tick } from 'svelte';
  import { X } from '@lucide/svelte';
  import ShortcutList from './ShortcutList.svelte';

  let dialog = $state<HTMLDivElement | null>(null);
  let previousFocus: HTMLElement | null = null;

  function close() {
    shortcutsOpen.set(false);
  }

  $effect(() => {
    if (!$shortcutsOpen || typeof document === 'undefined') return;
    previousFocus = document.activeElement as HTMLElement | null;
    void tick().then(() => dialog?.querySelector<HTMLElement>('button')?.focus());
    return () => previousFocus?.focus();
  });

  function onKeydown(e: KeyboardEvent) {
    if (!$shortcutsOpen) return;
    if (e.key === 'Escape') {
      e.preventDefault();
      close();
      return;
    }
    if (e.key !== 'Tab' || !dialog) return;
    const items = Array.from(dialog.querySelectorAll<HTMLElement>('button, [href], [tabindex]:not([tabindex="-1"])'));
    const first = items[0];
    const last = items.at(-1);
    if (e.shiftKey && document.activeElement === first) {
      e.preventDefault();
      last?.focus();
    } else if (!e.shiftKey && document.activeElement === last) {
      e.preventDefault();
      first?.focus();
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

{#if $shortcutsOpen}
  <div class="backdrop">
    <button class="backdrop-close" onclick={close} aria-label="Close keyboard shortcuts"></button>
    <div class="dialog card" role="dialog" aria-modal="true" aria-labelledby="shortcuts-title" bind:this={dialog}>
      <div class="head">
        <h2 id="shortcuts-title">Keyboard shortcuts</h2>
        <button class="x" onclick={close} aria-label="Close"><X size={16} /></button>
      </div>
      <ShortcutList />
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 80;
    background: rgba(0, 0, 0, 0.45);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
  }
  .backdrop-close {
    position: absolute;
    inset: 0;
    border: 0;
    background: transparent;
    cursor: default;
  }
  .dialog {
    position: relative;
    z-index: 1;
    width: min(440px, 100%);
    padding: 18px 20px 20px;
    border-radius: var(--radius-dialog);
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 12px;
  }
  .head h2 {
    font-size: 1.1rem;
  }
  .x {
    display: inline-flex;
    padding: 4px;
    border: none;
    background: transparent;
    color: var(--text-muted);
    border-radius: var(--radius-sm);
    line-height: 0;
  }
  .x:hover {
    background: var(--surface-hover);
    color: var(--text);
  }
</style>

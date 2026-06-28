<script module lang="ts">
  import { writable } from 'svelte/store';
  export const shortcutsOpen = writable(false);
  export function toggleShortcuts() {
    shortcutsOpen.update((v) => !v);
  }
</script>

<script lang="ts">
  import { X } from '@lucide/svelte';

  const shortcuts: { keys: string[]; desc: string }[] = [
    { keys: ['Ctrl', 'K'], desc: 'Focus search (or /)' },
    { keys: ['Ctrl', '1'], desc: 'Go to Discover' },
    { keys: ['Ctrl', '2'], desc: 'Go to Installed' },
    { keys: ['Ctrl', '3'], desc: 'Go to Settings' },
    { keys: ['↑', '↓', '←', '→'], desc: 'Move between search results' },
    { keys: ['Enter'], desc: 'Open or install the focused result' },
    { keys: ['Esc'], desc: 'Clear search, go back, or close a dialog' },
    { keys: ['Right-click'], desc: 'App actions menu (cards and installed rows)' },
    { keys: ['?'], desc: 'Show this help' }
  ];

  function close() {
    shortcutsOpen.set(false);
  }
</script>

<svelte:window onkeydown={(e) => $shortcutsOpen && e.key === 'Escape' && close()} />

{#if $shortcutsOpen}
  <div class="backdrop">
    <div class="dialog card" role="dialog" aria-modal="true">
      <div class="head">
        <h2>Keyboard shortcuts</h2>
        <button class="x" onclick={close} aria-label="Close"><X size={16} /></button>
      </div>
      <ul class="list">
        {#each shortcuts as s (s.desc)}
          <li>
            <span class="keys">
              {#each s.keys as k (k)}<kbd>{k}</kbd>{/each}
            </span>
            <span class="desc">{s.desc}</span>
          </li>
        {/each}
      </ul>
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
  .dialog {
    width: min(440px, 100%);
    padding: 18px 20px 20px;
    box-shadow: var(--shadow);
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
  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .list li {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 7px 0;
    border-bottom: 1px solid var(--border);
  }
  .list li:last-child {
    border-bottom: none;
  }
  .keys {
    flex: 0 0 130px;
    display: flex;
    gap: 4px;
    flex-wrap: wrap;
  }
  kbd {
    font-family: var(--font-mono);
    font-size: 0.72rem;
    color: var(--text);
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 2px 7px;
    line-height: 1.4;
  }
  .desc {
    flex: 1;
    font-size: 0.88rem;
    color: var(--text-muted);
  }
</style>

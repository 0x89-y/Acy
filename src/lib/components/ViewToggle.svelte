<script lang="ts">
  import { LayoutGrid, List } from '@lucide/svelte';
  import type { ViewMode } from '$lib/stores/settings';

  let { value, onChange }: { value: ViewMode; onChange: (v: ViewMode) => void } = $props();
</script>

<div class="view-toggle">
  <button class:on={value === 'grid'} onclick={() => onChange('grid')} title="Grid view" aria-label="Grid view">
    <LayoutGrid size={16} />
  </button>
  <button class:on={value === 'list'} onclick={() => onChange('list')} title="List view" aria-label="List view">
    <List size={16} />
  </button>
</div>

<style>
  .view-toggle {
    display: inline-flex;
    gap: 2px;
    padding: 2px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    flex-shrink: 0;
  }
  .view-toggle button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 5px 8px;
    border: none;
    background: transparent;
    color: var(--text-muted);
    border-radius: var(--radius-sm);
    line-height: 0;
  }
  .view-toggle button:hover {
    color: var(--text);
  }
  .view-toggle button.on {
    background: var(--accent);
    color: var(--accent-contrast);
  }
</style>

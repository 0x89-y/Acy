<script lang="ts">
  import { LayoutGrid, List } from '@lucide/svelte';
  import type { ViewMode } from '$lib/stores/settings';

  let { value, onChange }: { value: ViewMode; onChange: (v: ViewMode) => void } = $props();

  // Shows the layout a click switches to.
  let next = $derived<ViewMode>(value === 'grid' ? 'list' : 'grid');
  let label = $derived(next === 'grid' ? 'Grid view' : 'List view');
</script>

<button class="btn view-toggle" onclick={() => onChange(next)} title={label} aria-label={label}>
  {#if next === 'grid'}<LayoutGrid size={16} />{:else}<List size={16} />{/if}
</button>

<style>
  .view-toggle {
    padding: 7px 9px;
    flex-shrink: 0;
    color: var(--text-muted);
  }
  .view-toggle:hover {
    color: var(--text);
  }
</style>

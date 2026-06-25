<script lang="ts">
  import type { Source } from '$lib/types';
  let { source }: { source: Source } = $props();
</script>

<span class="badge {source}" title="Source: {source}">{source}</span>

<style>
  .badge {
    font-family: var(--font-mono);
    font-size: 0.68rem;
    padding: 1px 7px;
    border-radius: var(--radius-pill);
    border: 1px solid currentColor;
    line-height: 1.5;
    white-space: nowrap;
  }
  .winget {
    color: var(--winget);
  }
  .scoop {
    color: var(--scoop);
  }
  .choco {
    color: var(--choco);
  }
</style>

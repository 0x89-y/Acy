<script lang="ts">
  import { X } from '@lucide/svelte';
  import { cancelQueued, type Op } from '$lib/stores/ops';

  // Every waiting op, in queue order, as one card instead of a toast each.
  let { ops }: { ops: Op[] } = $props();

  const PREVIEW = 3;
  let expanded = $state(false);
  let shown = $derived(expanded ? ops : ops.slice(0, PREVIEW));
  let hidden = $derived(ops.length - shown.length);
</script>

<div class="queued card">
  <div class="head">
    <span class="title">{ops.length} waiting</span>
  </div>
  <ul class="items">
    {#each shown as op (op.id)}
      <li>
        <span class="name">{op.title}</span>
        <button
          class="icon-btn"
          onclick={() => cancelQueued(op.id)}
          title="Remove from queue"
          aria-label={`Remove ${op.title} from queue`}
        >
          <X size={14} />
        </button>
      </li>
    {/each}
  </ul>
  {#if ops.length > PREVIEW}
    <button class="more" onclick={() => (expanded = !expanded)}>
      {expanded ? 'Show less' : `Show ${hidden} more`}
    </button>
  {/if}
</div>

<style>
  .queued {
    display: flex;
    flex-direction: column;
    overflow: hidden;
    box-shadow: var(--shadow);
  }
  .head {
    padding: 10px 12px 6px;
  }
  .title {
    font-weight: 600;
    font-size: 0.9rem;
  }
  .items {
    list-style: none;
    margin: 0;
    padding: 0 6px 6px 12px;
    max-height: 30vh;
    overflow-y: auto;
  }
  li {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 28px;
  }
  .name {
    flex: 1;
    min-width: 0;
    font-size: 0.84rem;
    color: var(--text-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .icon-btn {
    display: inline-flex;
    flex-shrink: 0;
    padding: 4px;
    border: none;
    background: transparent;
    color: var(--text-muted);
    border-radius: var(--radius-sm);
    line-height: 0;
  }
  .icon-btn:hover {
    background: var(--surface-hover);
    color: var(--text);
  }
  .more {
    align-self: flex-start;
    margin: 0 0 8px 6px;
    padding: 4px 6px;
    border: none;
    background: none;
    color: var(--text-muted);
    font-size: 0.76rem;
    border-radius: var(--radius-sm);
  }
  .more:hover {
    color: var(--text);
    background: var(--surface-hover);
  }
</style>

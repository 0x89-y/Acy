<script lang="ts">
  import { activity, clearActivity, type ActivityAction } from '$lib/stores/activity';

  const labels: Record<ActivityAction, string> = {
    install: 'Installed',
    update: 'Updated',
    uninstall: 'Removed',
    'update-all': 'Updated all',
    setup: 'Set up'
  };

  const sameDay = (a: Date, b: Date) =>
    a.getFullYear() === b.getFullYear() && a.getMonth() === b.getMonth() && a.getDate() === b.getDate();

  function timeLabel(at: number): string {
    const d = new Date(at);
    const today = new Date();
    const yesterday = new Date();
    yesterday.setDate(today.getDate() - 1);
    const time = d.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' });
    if (sameDay(d, today)) return `Today, ${time}`;
    if (sameDay(d, yesterday)) return `Yesterday, ${time}`;
    return `${d.toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: 'numeric' })}, ${time}`;
  }
</script>

<div class="pane-head">
  <span class="muted total">
    {$activity.length} {$activity.length === 1 ? 'entry' : 'entries'}
  </span>
  <div class="spacer"></div>
  {#if $activity.length > 0}
    <button class="btn btn-ghost" onclick={clearActivity}>Clear activity</button>
  {/if}
</div>

<div class="pane-scroll">
  {#if $activity.length === 0}
    <p class="empty muted">Your installs, updates and removals will show up here.</p>
  {:else}
    <div class="entries">
      {#each $activity as entry (entry.id)}
        <div class="entry">
          <span class="dot" class:ok={entry.ok} class:bad={!entry.ok}></span>
          <div class="entry-main">
            <span><strong>{labels[entry.action]}</strong> {entry.name}</span>
            {#if entry.source}<span class="source mono muted">{entry.source}</span>{/if}
          </div>
          <time class="mono muted" datetime={new Date(entry.at).toISOString()}>{timeLabel(entry.at)}</time>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .pane-head {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 34px;
    padding: 10px 14px;
    border-bottom: 1px solid var(--border);
  }
  .total {
    font-size: 0.85rem;
  }
  .spacer {
    flex: 1;
  }
  .pane-scroll {
    flex: 1;
    min-height: 0;
    overflow: hidden auto;
  }
  .empty {
    padding: 16px 14px;
    font-size: 0.9rem;
  }
  .entries {
    display: flex;
    flex-direction: column;
  }
  .entry {
    display: flex;
    align-items: center;
    gap: 11px;
    padding: 11px 14px;
    border-top: 1px solid var(--border);
    font-size: 0.88rem;
  }
  .entry:first-child {
    border-top: none;
  }
  .dot {
    width: 8px;
    height: 8px;
    flex-shrink: 0;
    border-radius: 50%;
  }
  .dot.ok {
    background: var(--success);
  }
  .dot.bad {
    background: var(--danger);
  }
  .entry-main {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: baseline;
    gap: 8px;
  }
  .source,
  time {
    font-size: 0.74rem;
  }
  time {
    flex-shrink: 0;
  }
</style>

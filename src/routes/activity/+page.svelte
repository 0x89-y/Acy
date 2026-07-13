<script lang="ts">
  import { ArrowLeft } from '@lucide/svelte';
  import { activity, clearActivity, type ActivityAction, type ActivityEntry } from '$lib/stores/activity';

  const labels: Record<ActivityAction, string> = {
    install: 'Installed',
    update: 'Updated',
    uninstall: 'Removed',
    'update-all': 'Updated all',
    setup: 'Set up'
  };

  const sameDay = (a: Date, b: Date) =>
    a.getFullYear() === b.getFullYear() && a.getMonth() === b.getMonth() && a.getDate() === b.getDate();

  function dayLabel(d: Date): string {
    const today = new Date();
    const yesterday = new Date();
    yesterday.setDate(today.getDate() - 1);
    if (sameDay(d, today)) return 'Today';
    if (sameDay(d, yesterday)) return 'Yesterday';
    return d.toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: 'numeric' });
  }

  // Group entries into calendar days, newest first (the store is already
  // newest-first, so insertion order gives us that for free).
  let days = $derived.by(() => {
    const map = new Map<string, { key: string; label: string; items: ActivityEntry[] }>();
    for (const e of $activity) {
      const d = new Date(e.at);
      const key = `${d.getFullYear()}-${d.getMonth()}-${d.getDate()}`;
      let group = map.get(key);
      if (!group) {
        group = { key, label: dayLabel(d), items: [] };
        map.set(key, group);
      }
      group.items.push(e);
    }
    return [...map.values()];
  });

  // 'all' or a day key.
  let selected = $state<'all' | string>('all');
  let visible = $derived(
    selected === 'all' ? $activity : (days.find((d) => d.key === selected)?.items ?? [])
  );
  let paneLabel = $derived(
    selected === 'all' ? 'All activity' : (days.find((d) => d.key === selected)?.label ?? 'Activity')
  );

  function timeLabel(at: number): string {
    const d = new Date(at);
    const time = d.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' });
    if (selected === 'all') {
      return `${d.toLocaleDateString(undefined, { month: 'short', day: 'numeric' })}, ${time}`;
    }
    return time;
  }

  function clear() {
    clearActivity();
    selected = 'all';
  }
</script>

<div class="browse-panel">
  <div class="browse-rail">
    <div class="rail-head">
      <a class="back-btn" href="/settings" title="Back" aria-label="Back"><ArrowLeft size={17} /></a>
      <span class="rail-title">Activity</span>
    </div>
    <div class="rail-links">
      <button class="rail-link" class:active={selected === 'all'} onclick={() => (selected = 'all')}>
        <span>All activity</span><span class="rail-count mono">{$activity.length}</span>
      </button>
      {#each days as d (d.key)}
        <button class="rail-link" class:active={selected === d.key} onclick={() => (selected = d.key)}>
          <span>{d.label}</span><span class="rail-count mono">{d.items.length}</span>
        </button>
      {/each}
    </div>
  </div>

  <div class="browse-main">
    <div class="pane-head">
      <span class="pane-title">{paneLabel}</span>
      <span class="rail-count mono">{visible.length}</span>
      <div class="spacer"></div>
      {#if $activity.length > 0}
        <button class="btn btn-ghost" onclick={clear}>Clear activity</button>
      {/if}
    </div>
    <div class="pane-scroll">
      {#if visible.length === 0}
        <p class="empty muted">No activity yet. Completed operations will appear here.</p>
      {:else}
        <div class="entries">
          {#each visible as entry (entry.id)}
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
  </div>
</div>

<style>
  .browse-panel {
    flex: 1;
    min-height: 0;
    display: flex;
    align-items: stretch;
    overflow: hidden;
    background: var(--surface);
  }
  .browse-rail {
    flex: 0 0 190px;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    border-right: 1px solid var(--border);
    background: var(--surface-2);
  }
  .rail-head {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
  }
  .rail-title {
    font-size: 0.95rem;
    font-weight: 600;
  }
  .back-btn {
    flex-shrink: 0;
    width: 30px;
    height: 30px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface);
    color: var(--text-muted);
    line-height: 0;
    text-decoration: none;
  }
  .back-btn:hover {
    background: var(--surface-hover);
    color: var(--text);
    border-color: var(--accent);
  }
  .rail-links {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
  }
  .rail-link {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    text-align: left;
    padding: 9px 14px;
    border: none;
    border-top: 1px solid var(--border);
    border-left: 2px solid transparent;
    border-radius: 0;
    background: transparent;
    color: var(--text-muted);
    font-size: 0.9rem;
    font-weight: 500;
  }
  .rail-link:first-child {
    border-top: none;
  }
  .rail-link:hover {
    background: var(--surface-hover);
    color: var(--text);
  }
  .rail-link.active {
    background: var(--surface);
    color: var(--text);
    border-left-color: var(--accent);
  }
  .rail-count {
    font-size: 0.72rem;
    color: var(--text-muted);
  }
  .rail-link.active .rail-count {
    color: var(--accent);
  }
  .browse-main {
    flex: 1;
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .pane-head {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 34px;
    padding: 10px 14px;
    border-bottom: 1px solid var(--border);
  }
  .pane-title {
    font-size: 0.95rem;
    font-weight: 600;
  }
  .spacer {
    flex: 1;
  }
  .pane-scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
  }
  .empty {
    padding: 20px 16px;
    font-size: 0.9rem;
  }
  /* Entries are a flush divided list, edge-to-edge in the pane. */
  .entries {
    display: flex;
    flex-direction: column;
  }
  .entry {
    display: flex;
    align-items: center;
    gap: 11px;
    padding: 11px 16px;
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
  @media (max-width: 620px) {
    .entry {
      align-items: flex-start;
    }
    .entry-main {
      flex-direction: column;
      gap: 2px;
    }
    time {
      max-width: 110px;
      text-align: right;
    }
  }
</style>

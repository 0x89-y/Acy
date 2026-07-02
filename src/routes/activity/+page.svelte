<script lang="ts">
  import { ArrowLeft, ChevronLeft, ChevronRight } from '@lucide/svelte';
  import { activity, clearActivity, type ActivityAction } from '$lib/stores/activity';

  const PAGE_SIZE = 20;
  const labels: Record<ActivityAction, string> = {
    install: 'Installed',
    update: 'Updated',
    uninstall: 'Removed',
    'update-all': 'Updated all',
    setup: 'Set up'
  };

  let page = $state(0);
  let pageCount = $derived(Math.max(1, Math.ceil($activity.length / PAGE_SIZE)));
  let entries = $derived($activity.slice(page * PAGE_SIZE, (page + 1) * PAGE_SIZE));

  function when(at: number) {
    return new Date(at).toLocaleString(undefined, {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit'
    });
  }

  function go(next: number) {
    page = Math.max(0, Math.min(pageCount - 1, next));
    window.scrollTo({ top: 0, behavior: 'smooth' });
  }

  function clear() {
    clearActivity();
    page = 0;
  }
</script>

<a class="back" href="/settings"><ArrowLeft size={16} /> Settings</a>

<header>
  <div>
    <h1>Activity</h1>
    <p class="muted">Recent installs, updates, removals, and setup operations.</p>
  </div>
  {#if $activity.length > 0}
    <button class="btn btn-ghost" onclick={clear}>Clear activity</button>
  {/if}
</header>

{#if $activity.length === 0}
  <div class="empty card">
    <h2>No activity yet</h2>
    <p class="muted">Completed operations will appear here.</p>
  </div>
{:else}
  <div class="activity card">
    {#each entries as entry (entry.id)}
      <div class="entry">
        <span class="dot" class:ok={entry.ok} class:bad={!entry.ok}></span>
        <div class="entry-main">
          <span><strong>{labels[entry.action]}</strong> {entry.name}</span>
          {#if entry.source}<span class="source mono muted">{entry.source}</span>{/if}
        </div>
        <time class="mono muted" datetime={new Date(entry.at).toISOString()}>{when(entry.at)}</time>
      </div>
    {/each}
  </div>

  <nav class="pagination" aria-label="Activity pages">
    <button class="btn" onclick={() => go(page - 1)} disabled={page === 0}>
      <ChevronLeft size={15} /> Newer
    </button>
    <span class="mono muted">Page {page + 1} of {pageCount}</span>
    <button class="btn" onclick={() => go(page + 1)} disabled={page === pageCount - 1}>
      Older <ChevronRight size={15} />
    </button>
  </nav>
{/if}

<style>
  .back {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    margin-bottom: 18px;
    color: var(--text-muted);
    font-size: 0.9rem;
    text-decoration: none;
  }
  .back:hover {
    color: var(--text);
  }
  header {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 16px;
    margin-bottom: 20px;
  }
  header p {
    margin: 5px 0 0;
    font-size: 0.9rem;
  }
  .activity {
    padding: 4px 18px;
  }
  .entry {
    display: flex;
    align-items: center;
    gap: 11px;
    padding: 11px 0;
    border-bottom: 1px solid var(--border);
    font-size: 0.88rem;
  }
  .entry:last-child {
    border-bottom: 0;
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
  .pagination {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin-top: 18px;
    font-size: 0.76rem;
  }
  .empty {
    padding: 34px;
    text-align: center;
  }
  .empty h2 {
    font-size: 1.05rem;
  }
  .empty p {
    margin: 7px 0 0;
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

<script lang="ts">
  import { ArrowLeft } from '@lucide/svelte';
  import { CHANGELOG, type ChangelogEntry } from '$lib/changelog';

  // Group releases into minor series (major.minor), newest first.
  let series = $derived.by(() => {
    const map = new Map<string, ChangelogEntry[]>();
    for (const e of CHANGELOG) {
      const key = e.version.split('.').slice(0, 2).join('.');
      if (!map.has(key)) map.set(key, []);
      map.get(key)!.push(e);
    }
    return [...map.entries()].map(([key, releases]) => ({ key, releases }));
  });

  // 'all' or a series key like '0.7'.
  let selected = $state('all');
  let visible = $derived(
    selected === 'all' ? CHANGELOG : (series.find((s) => s.key === selected)?.releases ?? CHANGELOG)
  );
  let paneLabel = $derived(selected === 'all' ? 'All releases' : `${selected}.x`);
</script>

<div class="browse-panel">
  <div class="browse-rail">
    <div class="rail-head">
      <a class="back-btn" href="/settings" title="Back" aria-label="Back"><ArrowLeft size={17} /></a>
      <span class="rail-title">Changelog</span>
    </div>
    <div class="rail-links">
      <button class="rail-link" class:active={selected === 'all'} onclick={() => (selected = 'all')}>
        <span>All releases</span><span class="rail-count mono">{CHANGELOG.length}</span>
      </button>
      {#each series as s (s.key)}
        <button class="rail-link" class:active={selected === s.key} onclick={() => (selected = s.key)}>
          <span>{s.key}.x</span><span class="rail-count mono">{s.releases.length}</span>
        </button>
      {/each}
    </div>
  </div>

  <div class="browse-main">
    <div class="pane-head">
      <span class="pane-title">{paneLabel}</span>
      <span class="rail-count mono">{visible.length}</span>
    </div>
    <div class="pane-scroll">
      <div class="releases">
        {#each visible as r (r.version)}
          <article class="release">
            <div class="release-head">
              <h2 class="mono">v{r.version}</h2>
              <time class="mono muted" datetime={r.date}>{r.date}</time>
            </div>
            <ul>
              {#each r.changes as change (change)}
                <li>{change}</li>
              {/each}
            </ul>
          </article>
        {/each}
      </div>
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
  .pane-scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
  }
  /* Releases are a flush divided list, edge-to-edge in the pane. */
  .releases {
    display: flex;
    flex-direction: column;
  }
  .release {
    padding: 18px 20px;
    border-top: 1px solid var(--border);
  }
  .release:first-child {
    border-top: none;
  }
  .release-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 10px;
  }
  .release h2 {
    font-size: 1rem;
  }
  .release time {
    font-size: 0.74rem;
  }
  .release ul {
    margin: 0;
    padding-left: 20px;
    max-width: 78ch;
    color: var(--text-muted);
    font-size: 0.88rem;
  }
  .release li + li {
    margin-top: 5px;
  }
</style>

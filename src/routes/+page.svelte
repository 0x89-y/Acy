<script lang="ts">
  import { onMount } from 'svelte';
  import { get } from 'svelte/store';
  import { Search } from '@lucide/svelte';
  import AppCard from '$lib/components/AppCard.svelte';
  import ManagerSetup from '$lib/components/ManagerSetup.svelte';
  import * as api from '$lib/api';
  import type { CuratedApp, CuratedFile, SearchHit, Source } from '$lib/types';
  import { enabledSources } from '$lib/stores/managers';
  import { settings } from '$lib/stores/settings';
  import { installedKeys, loadInstalled } from '$lib/stores/library';

  let query = $state('');
  let curated = $state<CuratedFile | null>(null);
  let results = $state<SearchHit[]>([]);
  let searching = $state(false);
  let searchError = $state<string | null>(null);
  let loadingCurated = $state(true);

  let debounce: ReturnType<typeof setTimeout> | undefined;
  let showSearch = $derived(query.trim().length > 0);

  const COLLAPSED = 4;
  let expanded = $state<Record<string, boolean>>({});

  let visibleCategories = $derived(
    (curated?.categories ?? [])
      .map((cat) => ({
        ...cat,
        apps: cat.apps.filter((a) => $settings.managers[a.source] !== false)
      }))
      .filter((cat) => cat.apps.length > 0)
  );

  let curatedMatches = $derived.by(() => {
    const q = query.trim().toLowerCase();
    if (!q || !curated) return [] as CuratedApp[];
    const seen = new Set<string>();
    const out: CuratedApp[] = [];
    for (const cat of curated.categories) {
      for (const app of cat.apps) {
        if ($settings.managers[app.source] === false) continue;
        const name = (app.name ?? app.id).toLowerCase();
        if (name.includes(q) || app.id.toLowerCase().includes(q)) {
          const k = `${app.source}:${app.id.toLowerCase()}`;
          if (!seen.has(k)) {
            seen.add(k);
            out.push(app);
          }
        }
      }
    }
    return out;
  });

  let curatedKeys = $derived(
    new Set(curatedMatches.map((a) => `${a.source}:${a.id.toLowerCase()}`))
  );

  let managerResults = $derived(
    results.filter(
      (hit) => !hit.variants.some((v) => curatedKeys.has(`${v.source}:${v.id.toLowerCase()}`))
    )
  );

  function key(source: Source, id: string) {
    return `${source}:${id.toLowerCase()}`;
  }

  onMount(async () => {
    try {
      curated = await api.getCurated();
    } catch (e) {
      console.error('curated load failed', e);
    } finally {
      loadingCurated = false;
    }
    loadInstalled();
  });

  function onInput() {
    clearTimeout(debounce);
    if (!query.trim()) {
      results = [];
      searching = false;
      return;
    }
    searching = true;
    debounce = setTimeout(runSearch, 320);
  }

  async function runSearch() {
    clearTimeout(debounce);
    const q = query.trim();
    if (!q) return;
    searching = true;
    searchError = null;
    try {
      results = await api.search(q, get(enabledSources));
    } catch (e) {
      searchError = String(e);
      results = [];
    } finally {
      searching = false;
    }
  }

  function hitInstalled(hit: SearchHit): boolean {
    return hit.variants.some((v) => $installedKeys.has(key(v.source, v.id)));
  }
</script>

<ManagerSetup />

<div class="search">
  <div class="search-box">
    <Search size={18} />
    <input
      placeholder="Search"
      bind:value={query}
      oninput={onInput}
      onkeydown={(e) => e.key === 'Enter' && runSearch()}
    />
  </div>
</div>

{#if showSearch}
  {#if curatedMatches.length > 0}
    <section class="res-section">
      {#if managerResults.length > 0 || searching}
        <h2 class="res-head">From your list</h2>
      {/if}
      <div class="grid">
        {#each curatedMatches as app (app.source + app.id)}
          <AppCard
            name={app.name ?? app.id}
            description={app.description}
            variants={[{ source: app.source, id: app.id }]}
            installed={$installedKeys.has(key(app.source, app.id))}
            sub={app.id}
            homepage={app.icon ?? app.homepage}
            onChanged={() => loadInstalled(true)}
          />
        {/each}
      </div>
    </section>
  {/if}

  {#if searching}
    <p class="muted">Searching…</p>
  {:else if searchError}
    <p class="error">{searchError}</p>
  {:else if managerResults.length > 0}
    <section class="res-section">
      {#if curatedMatches.length > 0}
        <h2 class="res-head">Other results</h2>
      {/if}
      <div class="grid">
        {#each managerResults as hit (hit.name + hit.variants[0].id)}
          <AppCard
            name={hit.name}
            description={hit.description}
            variants={hit.variants.map((v) => ({ source: v.source, id: v.id }))}
            installed={hitInstalled(hit)}
            onChanged={() => loadInstalled(true)}
          />
        {/each}
      </div>
    </section>
  {:else if curatedMatches.length === 0}
    <p class="muted">No results for “{query}”.</p>
  {/if}
{:else if loadingCurated}
  <p class="muted">Loading…</p>
{:else}
  {#each visibleCategories as cat (cat.id)}
    <section class="cat">
      <div class="cat-head">
        <h2>{cat.title}</h2>
        {#if cat.apps.length > COLLAPSED}
          <button class="more-btn" onclick={() => (expanded[cat.id] = !expanded[cat.id])}>
            {expanded[cat.id] ? 'Show less' : `Show ${cat.apps.length - COLLAPSED} more`}
          </button>
        {/if}
      </div>
      <div class="grid">
        {#each expanded[cat.id] ? cat.apps : cat.apps.slice(0, COLLAPSED) as app (app.source + app.id)}
          <AppCard
            name={app.name ?? app.id}
            description={app.description}
            variants={[{ source: app.source, id: app.id }]}
            installed={$installedKeys.has(key(app.source, app.id))}
            sub={app.id}
            homepage={app.icon ?? app.homepage}
            onChanged={() => loadInstalled(true)}
          />
        {/each}
      </div>
    </section>
  {/each}
{/if}

<style>
  .search {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 26px;
    flex-wrap: wrap;
  }
  .search-box {
    display: flex;
    align-items: center;
    gap: 10px;
    flex: 1;
    min-width: 260px;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    padding: 0 14px;
    color: var(--text-muted);
  }
  .search-box:focus-within {
    border-color: var(--accent);
  }
  .search-box input {
    flex: 1;
    border: none;
    background: transparent;
    color: var(--text);
    padding: 12px 0;
    outline: none;
  }
  .cat {
    margin-bottom: 30px;
  }
  .cat-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 12px;
  }
  .cat h2 {
    font-size: 1.05rem;
  }
  .more-btn {
    background: none;
    border: none;
    color: var(--accent);
    font-size: 0.85rem;
    cursor: pointer;
    padding: 2px 4px;
    border-radius: var(--radius-sm);
    flex-shrink: 0;
  }
  .more-btn:hover {
    text-decoration: underline;
  }
  .res-section {
    margin-bottom: 24px;
  }
  .res-head {
    font-size: 0.92rem;
    font-weight: 600;
    color: var(--text-muted);
    margin-bottom: 12px;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
    gap: 14px;
  }
  .error {
    color: var(--danger);
    font-family: var(--font-mono);
    font-size: 0.85rem;
    white-space: pre-wrap;
  }
</style>

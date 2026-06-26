<script lang="ts">
  import { onMount } from 'svelte';
  import { get } from 'svelte/store';
  import { Search } from '@lucide/svelte';
  import AppCard from '$lib/components/AppCard.svelte';
  import ManagerSetup from '$lib/components/ManagerSetup.svelte';
  import * as api from '$lib/api';
  import type { CuratedApp, CuratedFile, SearchHit, Source, Variant } from '$lib/types';
  import { enabledSources, managers } from '$lib/stores/managers';
  import { settings } from '$lib/stores/settings';
  import { installedKeys, loadInstalled } from '$lib/stores/library';

  // Full install options for a curated app — its primary source plus alternates —
  // limited to sources the user hasn't disabled, de-duplicated by source.
  function curatedVariants(app: CuratedApp, managers: Record<Source, boolean>): Variant[] {
    const seen = new Set<Source>();
    const out: Variant[] = [];
    for (const v of [{ source: app.source, id: app.id }, ...app.alternates]) {
      if (managers[v.source] === false || seen.has(v.source)) continue;
      seen.add(v.source);
      out.push(v);
    }
    return out;
  }

  function anyInstalled(variants: Variant[]): boolean {
    return variants.some((v) => $installedKeys.has(key(v.source, v.id)));
  }

  let query = $state('');
  let searchInput = $state<HTMLInputElement | null>(null);

  // True once detection has run and reported every manager as unavailable.
  let noManagers = $derived($managers.length > 0 && $managers.every((m) => !m.available));
  let curated = $state<CuratedFile | null>(null);
  let results = $state<SearchHit[]>([]);
  let searching = $state(false);
  let searchError = $state<string | null>(null);
  let loadingCurated = $state(true);

  let searchedQuery = $state('');
  let trimmed = $derived(query.trim());
  let showSearch = $derived(trimmed.length > 0);
  // True once the loaded manager results match the current query (i.e. the user
  // pressed Enter / Search for exactly what's in the box).
  let searchedCurrent = $derived(showSearch && searchedQuery === trimmed);

  // Collapse long categories to the first few, with a per-category toggle.
  const COLLAPSED = 4;
  let expanded = $state<Record<string, boolean>>({});

  // Hide curated apps whose manager is disabled, and drop empty categories.
  let visibleCategories = $derived(
    (curated?.categories ?? [])
      .map((cat) => ({
        ...cat,
        apps: cat.apps.filter((a) => curatedVariants(a, $settings.managers).length > 0)
      }))
      .filter((cat) => cat.apps.length > 0)
  );

  // Curated apps matching the query, shown before the package-manager results.
  let curatedMatches = $derived.by(() => {
    const q = query.trim().toLowerCase();
    if (!q || !curated) return [] as CuratedApp[];
    const seen = new Set<string>();
    const out: CuratedApp[] = [];
    for (const cat of curated.categories) {
      for (const app of cat.apps) {
        if (curatedVariants(app, $settings.managers).length === 0) continue;
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
    new Set(
      curatedMatches.flatMap((a) =>
        [{ source: a.source, id: a.id }, ...a.alternates].map(
          (v) => `${v.source}:${v.id.toLowerCase()}`
        )
      )
    )
  );

  // Package-manager results, minus anything already shown from the curated list.
  let managerResults = $derived(
    results.filter(
      (hit) => !hit.variants.some((v) => curatedKeys.has(`${v.source}:${v.id.toLowerCase()}`))
    )
  );

  function key(source: Source, id: string) {
    return `${source}:${id.toLowerCase()}`;
  }

  onMount(() => {
    window.addEventListener('keydown', onShortcut);

    (async () => {
      try {
        curated = await api.getCurated();
      } catch (e) {
        console.error('curated load failed', e);
      } finally {
        loadingCurated = false;
      }
      loadInstalled();
    })();

    return () => window.removeEventListener('keydown', onShortcut);
  });

  // Ctrl/⌘+K, or "/" when not already typing, jumps to the search box.
  function onShortcut(e: KeyboardEvent) {
    const target = e.target as HTMLElement | null;
    const typing =
      target?.tagName === 'INPUT' ||
      target?.tagName === 'TEXTAREA' ||
      target?.isContentEditable;
    const cmdK = (e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k';
    const slash = e.key === '/' && !typing;
    if (cmdK || slash) {
      e.preventDefault();
      searchInput?.focus();
      searchInput?.select();
    }
  }

  // Typing only filters the curated list (derived live); managers aren't queried
  // until the user explicitly searches.
  function onInput() {
    searchError = null;
  }

  async function runSearch() {
    const q = query.trim();
    if (!q) return;
    searching = true;
    searchError = null;
    try {
      results = await api.search(q, get(enabledSources));
      searchedQuery = q;
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
      bind:this={searchInput}
      placeholder="Search curated apps…"
      bind:value={query}
      oninput={onInput}
      onkeydown={(e) => e.key === 'Enter' && runSearch()}
    />
    <kbd class="kbd">Ctrl K</kbd>
  </div>
  <button class="btn btn-accent search-btn" onclick={runSearch} disabled={!showSearch || searching}>
    {searching ? 'Searching…' : 'Search managers'}
  </button>
</div>

{#if showSearch}
  {#if curatedMatches.length > 0}
    <section class="res-section">
      <h2 class="res-head">From your list</h2>
      <div class="grid">
        {#each curatedMatches as app (app.source + app.id)}
          {@const vs = curatedVariants(app, $settings.managers)}
          <AppCard
            name={app.name ?? app.id}
            description={app.description}
            variants={vs}
            installed={anyInstalled(vs)}
            sub={app.id}
            homepage={app.icon ?? app.homepage}
            allowPick
            onChanged={() => loadInstalled(true)}
          />
        {/each}
      </div>
    </section>
  {/if}

  {#if searching}
    <p class="muted">Searching package managers…</p>
  {:else if searchError}
    <p class="error">{searchError}</p>
  {:else if searchedCurrent}
    {#if managerResults.length > 0}
      <section class="res-section">
        <h2 class="res-head">From package managers</h2>
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
    {:else}
      <p class="muted">
        {curatedMatches.length === 0
          ? `No results for “${query}”.`
          : `No package-manager results for “${query}”.`}
      </p>
    {/if}
  {:else}
    <div class="search-prompt">
      {#if curatedMatches.length === 0}
        <p class="muted">No curated apps match “{query}”.</p>
      {/if}
      <button class="btn" onclick={runSearch}>
        Search package managers for “{trimmed}”
      </button>
    </div>
  {/if}
{:else if loadingCurated}
  <p class="muted">Loading…</p>
{:else if noManagers}
  <div class="empty card">
    <h2>No package managers found</h2>
    <p class="muted">
      Acy works on top of winget, Scoop, or Chocolatey, and none were detected on this machine.
      Install one from Settings to start discovering and installing apps.
    </p>
    <a class="btn btn-accent" href="/settings">Open Settings</a>
  </div>
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
          {@const vs = curatedVariants(app, $settings.managers)}
          <AppCard
            name={app.name ?? app.id}
            description={app.description}
            variants={vs}
            installed={anyInstalled(vs)}
            sub={app.id}
            homepage={app.icon ?? app.homepage}
            allowPick
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
  .kbd {
    font-family: var(--font-mono);
    font-size: 0.7rem;
    color: var(--text-muted);
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 2px 7px;
    flex-shrink: 0;
  }
  .search-btn {
    flex-shrink: 0;
    white-space: nowrap;
  }
  .search-prompt {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 10px;
    padding: 8px 0 4px;
  }
  .empty {
    padding: 40px 32px;
    text-align: center;
    max-width: 520px;
    margin: 40px auto;
  }
  .empty h2 {
    font-size: 1.1rem;
    margin-bottom: 10px;
  }
  .empty p {
    font-size: 0.92rem;
    margin: 0 auto 20px;
    max-width: 420px;
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

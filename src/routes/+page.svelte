<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { get } from 'svelte/store';
  import { Check, ChevronDown, Funnel, Search, X } from '@lucide/svelte';
  import AppCard from '$lib/components/AppCard.svelte';
  import ManagerSetup from '$lib/components/ManagerSetup.svelte';
  import ViewToggle from '$lib/components/ViewToggle.svelte';
  import * as api from '$lib/api';
  import type { CuratedApp, CuratedFile, SearchHit, Source, Variant } from '$lib/types';
  import { enabledSources, managers } from '$lib/stores/managers';
  import { settings, setDiscoverView } from '$lib/stores/settings';
  import { installedKeys, loadInstalled } from '$lib/stores/library';
  import { runOp, summarizeBatch } from '$lib/install';

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
  // Card container class for the current view (grid tiles vs vertical list).
  let gridClass = $derived($settings.discoverView === 'list' ? 'list-flow' : 'grid');
  // True once the loaded manager results match the current query (i.e. the user
  // pressed Enter / Search for exactly what's in the box).
  let searchedCurrent = $derived(showSearch && searchedQuery === trimmed);

  // Keep Discover compact; long categories open on their own page.
  const COLLAPSED = 4;

  // ---- Tag filter (home browse) ----
  let allTags = $derived.by(() => {
    const set = new Set<string>();
    for (const cat of curated?.categories ?? [])
      for (const app of cat.apps)
        if (curatedVariants(app, $settings.managers).length > 0)
          for (const t of app.tags ?? []) set.add(t);
    return [...set].sort();
  });
  let activeTags = $state<Set<string>>(new Set());
  let tagMatchMode = $state<'all' | 'any'>('all');
  let tagMenuOpen = $state(false);
  let tagQuery = $state('');
  let tagFilterRoot = $state<HTMLDivElement | null>(null);
  let tagSearchInput = $state<HTMLInputElement | null>(null);
  let activeTagList = $derived([...activeTags].sort());
  let matchingTagOptions = $derived(
    allTags.filter((tag) => tag.toLowerCase().includes(tagQuery.trim().toLowerCase()))
  );
  let tagCounts = $derived.by(() => {
    const counts = new Map<string, number>();
    for (const cat of curated?.categories ?? []) {
      for (const app of cat.apps) {
        if (curatedVariants(app, $settings.managers).length === 0) continue;
        for (const tag of new Set(app.tags ?? [])) counts.set(tag, (counts.get(tag) ?? 0) + 1);
      }
    }
    return counts;
  });

  function toggleTag(t: string) {
    const next = new Set(activeTags);
    if (next.has(t)) next.delete(t);
    else next.add(t);
    activeTags = next;
  }
  function matchesTags(app: CuratedApp): boolean {
    if (activeTags.size === 0) return true;
    const t = new Set(app.tags ?? []);
    return tagMatchMode === 'all'
      ? activeTagList.every((tag) => t.has(tag))
      : activeTagList.some((tag) => t.has(tag));
  }
  function clearTags() {
    activeTags = new Set();
  }
  async function toggleTagMenu() {
    tagMenuOpen = !tagMenuOpen;
    if (tagMenuOpen) {
      tagQuery = '';
      await tick();
      tagSearchInput?.focus();
    }
  }
  function onWindowClick(e: MouseEvent) {
    if (tagMenuOpen && !tagFilterRoot?.contains(e.target as Node)) tagMenuOpen = false;
  }

  // Hide curated apps whose manager is disabled or that don't match the active
  // tag filter, and drop empty categories.
  let visibleCategories = $derived(
    (curated?.categories ?? [])
      .map((cat) => ({
        ...cat,
        apps: cat.apps.filter(
          (a) => curatedVariants(a, $settings.managers).length > 0 && matchesTags(a)
        )
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

  // ---- Recent searches ----
  const RECENT_KEY = 'acy-recent-searches';
  function loadRecent(): string[] {
    try {
      const r = localStorage.getItem(RECENT_KEY);
      const v = r ? JSON.parse(r) : [];
      return Array.isArray(v) ? v.slice(0, 6) : [];
    } catch {
      return [];
    }
  }
  let recent = $state<string[]>(loadRecent());
  let searchFocused = $state(false);
  let showRecent = $derived(searchFocused && !trimmed && recent.length > 0);
  function recordSearch(q: string) {
    recent = [q, ...recent.filter((r) => r.toLowerCase() !== q.toLowerCase())].slice(0, 6);
    try {
      localStorage.setItem(RECENT_KEY, JSON.stringify(recent));
    } catch {
      // ignore quota errors
    }
  }
  function useRecent(q: string) {
    query = q;
    searchInput?.focus();
    runSearch();
  }

  // ---- Multi-select install ----
  let selectMode = $state(false);
  let installing = $state(false);
  let installProgress = $state<{ current: number; total: number; name: string } | null>(null);
  let selectedApps = $state<Map<string, { name: string; variants: Variant[] }>>(new Map());
  function appKey(app: CuratedApp) {
    return `${app.source}:${app.id}`;
  }
  function chosenVariant(vs: Variant[]): Variant {
    return (
      ($settings.preferredSource && vs.find((v) => v.source === $settings.preferredSource)) || vs[0]
    );
  }
  function toggleSelectApp(app: CuratedApp) {
    const vs = curatedVariants(app, $settings.managers);
    if (vs.length === 0 || anyInstalled(vs)) return;
    const k = appKey(app);
    const next = new Map(selectedApps);
    if (next.has(k)) next.delete(k);
    else next.set(k, { name: app.name ?? app.id, variants: vs });
    selectedApps = next;
  }
  function exitSelect() {
    selectMode = false;
    selectedApps = new Map();
  }
  async function installSelected() {
    installing = true;
    const total = selectedApps.size;
    let ok = 0;
    let current = 0;
    for (const entry of selectedApps.values()) {
      current++;
      installProgress = { current, total, name: entry.name };
      const v = chosenVariant(entry.variants);
      if (await runOp('install', v.source, v.id, entry.name)) ok++;
    }
    installProgress = null;
    installing = false;
    exitSelect();
    loadInstalled(true);
    if (total > 1) summarizeBatch(total, ok, 'installed');
  }

  // ---- Keyboard navigation of result cards ----
  let resultsEl = $state<HTMLElement | null>(null);
  function resultCards(): HTMLElement[] {
    return resultsEl ? Array.from(resultsEl.querySelectorAll<HTMLElement>('a.main')) : [];
  }
  function onResultsKey(e: KeyboardEvent) {
    if (!showSearch || !e.key.startsWith('Arrow')) return;
    const active = document.activeElement as HTMLElement | null;
    const inInput = active?.tagName === 'INPUT' || active?.tagName === 'TEXTAREA';
    const items = resultCards();
    if (items.length === 0) return;
    if (inInput) {
      if (e.key === 'ArrowDown' && active === searchInput) {
        e.preventDefault();
        items[0].focus();
      }
      return;
    }
    let cols = items.length;
    if (items.length > 1) {
      const top0 = items[0].offsetTop;
      const firstWrap = items.findIndex((it) => it.offsetTop !== top0);
      cols = firstWrap === -1 ? items.length : Math.max(1, firstWrap);
    }
    const idx = items.indexOf(active as HTMLElement);
    if (idx === -1) {
      e.preventDefault();
      items[0].focus();
      return;
    }
    let next = idx;
    if (e.key === 'ArrowRight') next++;
    else if (e.key === 'ArrowLeft') next--;
    else if (e.key === 'ArrowDown') next += cols;
    else if (e.key === 'ArrowUp') next -= cols;
    next = Math.max(0, Math.min(items.length - 1, next));
    if (next !== idx) {
      e.preventDefault();
      items[next].focus();
    }
  }

  onMount(() => {
    window.addEventListener('keydown', onShortcut);
    window.addEventListener('keydown', onResultsKey);

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

    return () => {
      window.removeEventListener('keydown', onShortcut);
      window.removeEventListener('keydown', onResultsKey);
    };
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
    if (e.key === 'Escape' && tagMenuOpen) {
      e.preventDefault();
      tagMenuOpen = false;
      return;
    }
    if (e.key === 'Escape' && selectMode && !typing) {
      e.preventDefault();
      exitSelect();
      return;
    }
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
      recordSearch(q);
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

<svelte:window onclick={onWindowClick} />

<ManagerSetup />

<div class="search">
  <div class="search-box">
    <Search size={18} />
    <input
      bind:this={searchInput}
      aria-label="Search apps"
      placeholder="Search apps…"
      bind:value={query}
      oninput={onInput}
      onfocus={() => (searchFocused = true)}
      onblur={() => setTimeout(() => (searchFocused = false), 120)}
      onkeydown={(e) => {
        if (e.key === 'Enter') runSearch();
        else if (e.key === 'Escape') {
          if (query) query = '';
          else searchInput?.blur();
        }
      }}
    />
    {#if query}
      <button class="clear" onclick={() => { query = ''; searchInput?.focus(); }} aria-label="Clear search">
        <X size={15} />
      </button>
    {:else}
      <kbd class="kbd">Ctrl K</kbd>
    {/if}
  </div>
  <button class="btn btn-accent search-btn" onclick={runSearch} disabled={!showSearch || searching}>
    {searching ? 'Searching…' : 'Search'}
  </button>
</div>

{#if showRecent}
  <div class="recents">
    <span class="recents-label muted">Recent</span>
    {#each recent as r (r)}
      <button class="recent" onmousedown={(e) => e.preventDefault()} onclick={() => useRecent(r)}>
        {r}
      </button>
    {/each}
  </div>
{/if}

{#if !loadingCurated && !noManagers}
  <div class="discover-bar">
    {#if !showSearch && allTags.length > 0}
      <div class="filter-wrap" bind:this={tagFilterRoot}>
        <button
          class="btn filter-trigger"
          class:on={activeTags.size > 0}
          onclick={toggleTagMenu}
          aria-haspopup="dialog"
          aria-expanded={tagMenuOpen}
        >
          <Funnel size={15} /> Filters
          {#if activeTags.size > 0}<span class="filter-count">{activeTags.size}</span>{/if}
          <ChevronDown size={14} />
        </button>

        {#if tagMenuOpen}
          <div class="filter-pop card" role="dialog" aria-label="Filter apps by tag">
            <div class="filter-head">
              <strong>Filter by tags</strong>
              {#if activeTags.size > 0}
                <button class="clear-tags" onclick={clearTags}>Clear all</button>
              {/if}
            </div>
            <input
              class="tag-search"
              bind:this={tagSearchInput}
              bind:value={tagQuery}
              placeholder="Find a tag…"
              aria-label="Find a tag"
            />
            <div class="match-mode" aria-label="Tag matching mode">
              <button class:on={tagMatchMode === 'all'} onclick={() => (tagMatchMode = 'all')}>
                Match all
              </button>
              <button class:on={tagMatchMode === 'any'} onclick={() => (tagMatchMode = 'any')}>
                Match any
              </button>
            </div>
            <div class="tag-options">
              {#each matchingTagOptions as tag (tag)}
                <button
                  class="tag-option"
                  class:on={activeTags.has(tag)}
                  onclick={() => toggleTag(tag)}
                  aria-pressed={activeTags.has(tag)}
                >
                  <span class="tag-check">{#if activeTags.has(tag)}<Check size={13} />{/if}</span>
                  <span>{tag}</span>
                  <span class="tag-count mono">{tagCounts.get(tag) ?? 0}</span>
                </button>
              {/each}
              {#if matchingTagOptions.length === 0}
                <span class="no-tags muted">No matching tags.</span>
              {/if}
            </div>
          </div>
        {/if}
      </div>

      {#each activeTagList.slice(0, 2) as tag (tag)}
        <button class="active-tag" onclick={() => toggleTag(tag)} title={`Remove ${tag} filter`}>
          {tag} <X size={12} />
        </button>
      {/each}
      {#if activeTagList.length > 2}
        <button
          class="active-more"
          onclick={(e) => {
            e.stopPropagation();
            toggleTagMenu();
          }}>+{activeTagList.length - 2}</button
        >
      {/if}
    {/if}
    <button
      class="btn sel-toggle"
      onclick={() => (selectMode ? exitSelect() : (selectMode = true))}
      aria-pressed={selectMode}
    >
      {selectMode ? 'Cancel selection' : 'Select apps'}
    </button>
    <div class="discover-spacer"></div>
    <ViewToggle value={$settings.discoverView} onChange={setDiscoverView} />
  </div>
{/if}

{#if selectMode && selectedApps.size > 0}
  <div class="sel-bar">
    <span class="sel-count" role="status" aria-live="polite">
      {#if installProgress}
        Installing {installProgress.current} of {installProgress.total} · {installProgress.name}
      {:else}
        {selectedApps.size} selected
      {/if}
    </span>
    <div class="sel-spacer"></div>
    <button class="btn btn-accent" onclick={installSelected} disabled={installing}>
      {installProgress ? `${installProgress.current} of ${installProgress.total}…` : `Install ${selectedApps.size}`}
    </button>
  </div>
{/if}

{#if showSearch}
  <div bind:this={resultsEl}>
  {#if curatedMatches.length > 0}
    <section class="res-section">
      <h2 class="res-head">From your list</h2>
      <div class={gridClass}>
        {#each curatedMatches as app (app.source + app.id)}
          {@const vs = curatedVariants(app, $settings.managers)}
          <AppCard
            name={app.name ?? app.id}
            description={app.description}
            variants={vs}
            installed={anyInstalled(vs)}
            homepage={app.icon ?? app.homepage}
            tags={app.tags ?? []}
            allowPick
            layout={$settings.discoverView}
            highlight={trimmed}
            selectable={selectMode && !anyInstalled(vs)}
            selected={selectedApps.has(appKey(app))}
            onToggleSelect={() => toggleSelectApp(app)}
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
        <div class={gridClass}>
          {#each managerResults as hit (hit.name + hit.variants[0].id)}
            <AppCard
              name={hit.name}
              description={hit.description}
              variants={hit.variants.map((v) => ({ source: v.source, id: v.id }))}
              installed={hitInstalled(hit)}
              layout={$settings.discoverView}
              highlight={trimmed}
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
      <p class="muted">Press Enter or select Search to look beyond the curated catalog.</p>
    </div>
  {/if}
  </div>
{:else if loadingCurated}
  <div class="grid">
    {#each Array(8) as _, i (i)}
      <div class="card sk-card">
        <div class="sk-top">
          <div class="skeleton sk-icon"></div>
          <div class="sk-lines">
            <div class="skeleton sk-line lg"></div>
            <div class="skeleton sk-line sm"></div>
          </div>
        </div>
        <div class="skeleton sk-line full"></div>
      </div>
    {/each}
  </div>
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
  {#if visibleCategories.length === 0 && activeTags.size > 0}
    <div class="filter-empty card">
      <h2>No apps match these filters</h2>
      <p class="muted">Try fewer tags or switch between matching all and matching any.</p>
      <button class="btn" onclick={clearTags}>Clear filters</button>
    </div>
  {:else}
    {#each visibleCategories as cat (cat.id)}
    <section class="cat">
      <div class="cat-head">
        <h2>{cat.title}</h2>
        {#if cat.apps.length > COLLAPSED}
          <a class="more-btn" href={`/category/${encodeURIComponent(cat.id)}`}>
            View all {cat.apps.length}
          </a>
        {/if}
      </div>
      <div class={gridClass}>
        {#each cat.apps.slice(0, COLLAPSED) as app (app.source + app.id)}
          {@const vs = curatedVariants(app, $settings.managers)}
          <AppCard
            name={app.name ?? app.id}
            description={app.description}
            variants={vs}
            installed={anyInstalled(vs)}
            homepage={app.icon ?? app.homepage}
            tags={app.tags ?? []}
            allowPick
            layout={$settings.discoverView}
            selectable={selectMode && !anyInstalled(vs)}
            selected={selectedApps.has(appKey(app))}
            onToggleSelect={() => toggleSelectApp(app)}
            onChanged={() => loadInstalled(true)}
          />
        {/each}
      </div>
    </section>
    {/each}
  {/if}
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
  .clear {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    padding: 4px;
    border: none;
    background: transparent;
    color: var(--text-muted);
    border-radius: var(--radius-sm);
    line-height: 0;
  }
  .clear:hover {
    background: var(--surface-hover);
    color: var(--text);
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
  .recents {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    margin: -14px 0 18px;
  }
  .recents-label {
    font-size: 0.78rem;
  }
  .recent {
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--text);
    padding: 4px 12px;
    border-radius: var(--radius-pill);
    font-size: 0.82rem;
  }
  .recent:hover {
    background: var(--surface-hover);
    border-color: var(--border-strong);
  }
  .discover-bar {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 10px;
    margin: -10px 0 16px;
  }
  .filter-wrap {
    position: relative;
  }
  .filter-trigger.on {
    color: var(--accent);
    border-color: var(--accent);
  }
  .filter-count {
    min-width: 18px;
    height: 18px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 0 5px;
    border-radius: var(--radius-pill);
    background: var(--accent);
    color: var(--accent-contrast);
    font-size: 0.7rem;
    font-weight: 600;
  }
  .filter-pop {
    position: absolute;
    z-index: 35;
    top: calc(100% + 7px);
    left: 0;
    width: min(330px, calc(100vw - 48px));
    padding: 14px;
    box-shadow: var(--shadow);
  }
  .filter-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 10px;
    font-size: 0.9rem;
  }
  .clear-tags {
    padding: 2px 4px;
    border: 0;
    background: transparent;
    color: var(--accent);
    font-size: 0.78rem;
  }
  .tag-search {
    width: 100%;
    padding: 8px 10px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    outline: none;
    background: var(--surface-2);
    color: var(--text);
    font-size: 0.84rem;
  }
  .tag-search:focus {
    border-color: var(--accent);
  }
  .match-mode {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 3px;
    padding: 3px;
    margin: 9px 0;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--surface-2);
  }
  .match-mode button {
    padding: 5px 8px;
    border: 0;
    border-radius: 5px;
    background: transparent;
    color: var(--text-muted);
    font-size: 0.77rem;
  }
  .match-mode button.on {
    background: var(--surface);
    color: var(--text);
    box-shadow: var(--card-shadow);
  }
  .tag-options {
    max-height: 245px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .tag-option {
    display: grid;
    grid-template-columns: 19px minmax(0, 1fr) auto;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 7px 8px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text);
    text-align: left;
    font-size: 0.82rem;
  }
  .tag-option:hover,
  .tag-option.on {
    background: var(--surface-hover);
  }
  .tag-check {
    width: 17px;
    height: 17px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: 1px solid var(--border-strong);
    border-radius: 5px;
    color: var(--accent-contrast);
  }
  .tag-option.on .tag-check {
    border-color: var(--accent);
    background: var(--accent);
  }
  .tag-count {
    color: var(--text-muted);
    font-size: 0.7rem;
  }
  .no-tags {
    padding: 12px 8px;
    font-size: 0.82rem;
  }
  .active-tag,
  .active-more {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    min-height: 28px;
    padding: 3px 9px;
    border: 1px solid color-mix(in srgb, var(--accent) 55%, var(--border));
    border-radius: var(--radius-pill);
    background: color-mix(in srgb, var(--accent) 10%, var(--surface));
    color: var(--accent);
    font-size: 0.76rem;
  }
  .active-more {
    min-width: 32px;
    justify-content: center;
  }
  .discover-spacer {
    flex: 1;
  }
  .list-flow {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .sel-toggle {
    font-size: 0.85rem;
  }
  .filter-empty {
    max-width: 480px;
    margin: 38px auto;
    padding: 30px;
    text-align: center;
  }
  .filter-empty h2 {
    font-size: 1.05rem;
  }
  .filter-empty p {
    margin: 7px 0 18px;
    font-size: 0.88rem;
  }
  .sel-bar {
    position: sticky;
    top: 64px;
    z-index: 15;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 14px;
    margin-bottom: 18px;
    background: color-mix(in srgb, var(--surface) 92%, transparent);
    backdrop-filter: blur(8px);
    border: 1px solid var(--accent);
    border-radius: var(--radius);
  }
  .sel-count {
    font-size: 0.9rem;
    font-weight: 600;
  }
  .sel-spacer {
    flex: 1;
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
  .sk-card {
    padding: 14px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .sk-top {
    display: flex;
    gap: 12px;
  }
  .sk-icon {
    width: 40px;
    height: 40px;
    border-radius: var(--radius-sm);
    flex-shrink: 0;
  }
  .sk-lines {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 7px;
    justify-content: center;
  }
  .sk-line {
    height: 11px;
  }
  .sk-line.lg {
    width: 62%;
  }
  .sk-line.sm {
    width: 40%;
  }
  .sk-line.full {
    width: 100%;
    height: 9px;
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

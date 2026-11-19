<script lang="ts">
  import { page } from '$app/stores';
  import { goto, afterNavigate } from '$app/navigation';
  import { get } from 'svelte/store';
  import { Check, CircleAlert } from '@lucide/svelte';
  import AppCard from '$lib/components/AppCard.svelte';
  import type { CuratedApp, SearchHit, Source } from '$lib/types';
  import { settings } from '$lib/stores/settings';
  import { installedKeys, loadInstalled } from '$lib/stores/library';
  import {
    curated,
    loadCurated,
    reloadCurated,
    addToCurated,
    searchHitToInput,
    curatedKeys,
    curatedKey
  } from '$lib/stores/curated';
  import { enabledSources } from '$lib/stores/managers';
  import { notice } from '$lib/stores/ops';
  import {
    searchQuery,
    managerSearch,
    searchManagers,
    loadRecentSearches,
    recordRecentSearch
  } from '$lib/stores/search';
  import { browseCategories, curatedVariants, uniqueApps, variantKey } from '$lib/catalog';

  const NAMES: Record<Source, string> = {
    winget: 'winget',
    scoop: 'Scoop',
    choco: 'Chocolatey',
    msstore: 'Microsoft Store'
  };

  $effect(() => {
    loadCurated();
    loadInstalled();
  });

  let q = $derived(($page.url.searchParams.get('q') ?? '').trim());
  let ql = $derived(q.toLowerCase());

  // Opening /search?q=… from a link, Back/Forward or on launch fills the box and
  // searches. Typing navigates with goto() and is skipped: the URL trails the box
  // by a keystroke, so syncing it back would overwrite what was just typed (and
  // search the package managers for the first letter).
  afterNavigate(({ type, to }) => {
    if (type === 'goto') return;
    const query = (to?.url.searchParams.get('q') ?? '').trim();
    if (query && get(searchQuery).trim() !== query) {
      searchQuery.set(query);
      searchManagers(query);
    }
  });

  let catalogApps = $derived(
    uniqueApps(browseCategories($curated, $settings.managers, $settings.showCuratedApps))
  );
  let catalogMatches = $derived.by(() => {
    if (!ql) return [] as CuratedApp[];
    return catalogApps.filter(
      (a) =>
        (a.name ?? a.id).toLowerCase().includes(ql) ||
        a.id.toLowerCase().includes(ql) ||
        (a.tags ?? []).some((t) => t.toLowerCase().includes(ql))
    );
  });
  // Package-manager hits already shown as a catalog match are left out.
  let catalogMatchKeys = $derived(
    new Set(catalogMatches.flatMap((a) => [a, ...a.alternates].map(variantKey)))
  );
  let current = $derived($managerSearch.query === q);
  let managerHits = $derived(
    current
      ? $managerSearch.hits.filter((h) => !h.variants.some((v) => catalogMatchKeys.has(variantKey(v))))
      : []
  );
  let statuses = $derived(
    $enabledSources.map((s) => ({ source: s, state: current ? $managerSearch.status[s] : undefined }))
  );
  let stillSearching = $derived(current && statuses.some((s) => s.state === 'searching'));
  let started = $derived(current && statuses.some((s) => s.state !== undefined));

  let inListKeys = $derived(curatedKeys($curated));
  const hitInList = (hit: SearchHit) =>
    hit.variants.some((v) => inListKeys.has(curatedKey(v.source, v.id)));
  const hitInstalled = (hit: SearchHit) =>
    hit.variants.some((v) => $installedKeys.has(variantKey(v)));
  const appVariants = (a: CuratedApp) => curatedVariants(a, $settings.managers);

  async function addHit(hit: SearchHit) {
    const res = await addToCurated(searchHitToInput(hit));
    if (res !== 'error') await reloadCurated();
    if (res === 'added') notice(`Added ${hit.name} to your list.`, 'ok');
    else if (res === 'exists') notice(`${hit.name} is already in your list.`, 'ok');
    else notice(`Couldn't add ${hit.name} to your list.`, 'error');
  }

  let recent = $state(loadRecentSearches());
  function useRecent(r: string) {
    recordRecentSearch(r);
    recent = loadRecentSearches();
    searchQuery.set(r);
    searchManagers(r);
    goto(`/search?q=${encodeURIComponent(r)}`);
  }

  let backTo = $derived(`/search?q=${encodeURIComponent(q)}`);
</script>

<div class="view">
  <header class="view-head">
    <div>
      <h1>{q ? `Results for “${q}”` : 'Search'}</h1>
      {#if q && started}
        <p class="view-sub sources" aria-live="polite">
          {#each statuses as s (s.source)}
            <span class="src">
              {#if s.state === 'searching'}
                <span class="spinner" aria-hidden="true"></span>
              {:else if s.state === 'done'}
                <Check size={13} class="ok" aria-hidden="true" />
              {:else if s.state === 'error'}
                <CircleAlert size={13} class="bad" aria-hidden="true" />
              {/if}
              {NAMES[s.source]}{s.state === 'searching' ? ' searching…' : s.state === 'error' ? ' failed' : ''}
            </span>
          {/each}
        </p>
      {/if}
    </div>
  </header>

  <div class="view-body">
    {#if !q}
      <div class="intro">
        <p class="muted">
          Type in the search box to find apps in your catalog. Press Enter to search the package
          managers too.
        </p>
        {#if recent.length > 0}
          <h2 class="sec">Recent searches</h2>
          <div class="recents">
            {#each recent as r (r)}
              <button class="recent" onclick={() => useRecent(r)}>{r}</button>
            {/each}
          </div>
        {/if}
      </div>
    {:else}
      {#if catalogMatches.length > 0}
        <section aria-labelledby="sec-catalog">
          <h2 class="sec" id="sec-catalog">From your catalog <span class="count mono">{catalogMatches.length}</span></h2>
          <div class="rows">
            {#each catalogMatches as app (variantKey(app))}
              {@const vs = appVariants(app)}
              <AppCard
                name={app.name ?? app.id}
                description={app.description}
                variants={vs}
                installed={vs.some((v) => $installedKeys.has(variantKey(v)))}
                homepage={app.icon ?? app.homepage}
                layout="list"
                highlight={q}
                {backTo}
                onChanged={() => loadInstalled(true)}
              />
            {/each}
          </div>
        </section>
      {/if}

      <section aria-labelledby="sec-managers">
        <h2 class="sec" id="sec-managers">
          From package managers
          {#if managerHits.length > 0}<span class="count mono">{managerHits.length}</span>{/if}
        </h2>
        {#if managerHits.length > 0}
          <div class="rows">
            {#each managerHits as hit (hit.name + hit.variants[0].source + hit.variants[0].id)}
              <AppCard
                name={hit.name}
                description={hit.description}
                variants={hit.variants.map((v) => ({ source: v.source, id: v.id }))}
                installed={hitInstalled(hit)}
                layout="list"
                highlight={q}
                {backTo}
                inList={hitInList(hit)}
                onAddToList={() => addHit(hit)}
                iconCacheOnly
                onChanged={() => loadInstalled(true)}
              />
            {/each}
          </div>
        {:else if stillSearching || !started}
          <p class="muted msg">
            {started ? 'Searching…' : 'Press Enter to search package managers.'}
          </p>
        {:else}
          <p class="muted msg">
            {catalogMatches.length === 0 ? `No results for “${q}”.` : `No other results from package managers.`}
          </p>
        {/if}
      </section>
    {/if}
  </div>
</div>

<style>
  .sources {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 14px;
    font-size: var(--fs-sm);
  }
  .src {
    display: inline-flex;
    align-items: center;
    gap: 5px;
  }
  .src :global(.ok) {
    color: var(--success);
  }
  .src :global(.bad) {
    color: var(--danger);
  }
  .spinner {
    width: 11px;
    height: 11px;
    border: 2px solid var(--border-strong);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  .sec {
    display: flex;
    align-items: baseline;
    gap: 8px;
    padding: 14px 24px 8px;
    font-size: var(--fs-body);
  }
  .count {
    font-size: var(--fs-xs);
    font-weight: 400;
    color: var(--text-muted);
  }
  .rows {
    display: flex;
    flex-direction: column;
    border-top: 1px solid var(--border);
    border-bottom: 1px solid var(--border);
  }
  .msg {
    padding: 4px 24px 16px;
    font-size: var(--fs-md);
  }
  .intro {
    padding: 0 24px;
    max-width: 640px;
  }
  .intro .sec {
    padding: 18px 0 8px;
  }
  .recents {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .recent {
    padding: 5px 12px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-pill);
    background: var(--surface);
    color: var(--text);
    font-size: var(--fs-sm);
  }
  .recent:hover {
    background: var(--surface-hover);
  }
</style>

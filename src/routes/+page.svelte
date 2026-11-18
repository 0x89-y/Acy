<script lang="ts">
  import { tick } from 'svelte';
  import { page } from '$app/stores';
  import { Check, Funnel, X, ArrowRight } from '@lucide/svelte';
  import AppCard from '$lib/components/AppCard.svelte';
  import AppIcon from '$lib/components/AppIcon.svelte';
  import ViewToggle from '$lib/components/ViewToggle.svelte';
  import type { CuratedApp } from '$lib/types';
  import { managers } from '$lib/stores/managers';
  import { settings, setDiscoverView, setSettingsTab } from '$lib/stores/settings';
  import { installedKeys, loadInstalled } from '$lib/stores/library';
  import {
    curated,
    loadCurated,
    reloadCurated,
    moveCuratedApp,
    catalog,
    downloadCatalog
  } from '$lib/stores/curated';
  import { queueApps, installApps, batch, type QueuedApp } from '$lib/stores/queue';
  import { confirmAction } from '$lib/stores/confirm';
  import { notice } from '$lib/stores/ops';
  import {
    browseCategories,
    curatedVariants,
    resolveCollections,
    uniqueApps,
    variantKey,
    type ResolvedCollection
  } from '$lib/catalog';
  import type { CtxItem } from '$lib/stores/contextMenu';

  $effect(() => {
    loadCurated();
    loadInstalled();
  });

  let params = $derived($page.url.searchParams);
  let view = $derived(
    params.get('collection')
      ? 'collection'
      : params.get('cat')
        ? 'category'
        : params.get('view') === 'all'
          ? 'all'
          : 'home'
  );

  let noManagers = $derived($managers.length > 0 && $managers.every((m) => !m.available));
  let categories = $derived(
    browseCategories($curated, $settings.managers, $settings.showCuratedApps)
  );
  let allApps = $derived(
    uniqueApps(categories).sort((a, b) => (a.name ?? a.id).localeCompare(b.name ?? b.id))
  );
  let collections = $derived(
    $settings.showCuratedApps ? resolveCollections($curated, categories) : []
  );

  const variantsOf = (app: CuratedApp) => curatedVariants(app, $settings.managers);
  const isInstalled = (app: CuratedApp) =>
    variantsOf(app).some((v) => $installedKeys.has(variantKey(v)));
  const toQueued = (app: CuratedApp): QueuedApp => ({
    key: variantKey(app),
    name: app.name ?? app.id,
    variants: variantsOf(app)
  });

  // --- Category / All / Collection views ---------------------------------
  let currentCategory = $derived(
    view === 'category' ? (categories.find((c) => c.id === params.get('cat')) ?? null) : null
  );
  let currentCollection = $derived(
    view === 'collection'
      ? (collections.find((c) => c.id === params.get('collection')) ?? null)
      : null
  );
  let baseApps = $derived(
    view === 'category'
      ? (currentCategory?.apps ?? [])
      : view === 'collection'
        ? (currentCollection?.apps ?? [])
        : allApps
  );

  // Tag filter (All apps and categories). A ?tag= link presets it.
  let activeTags = $state<Set<string>>(new Set());
  let tagMatchMode = $state<'all' | 'any'>('all');
  $effect(() => {
    const t = params.get('tag');
    activeTags = t ? new Set([t]) : new Set();
  });
  let allTags = $derived([...new Set(baseApps.flatMap((a) => a.tags ?? []))].sort());
  let tagCounts = $derived.by(() => {
    const m = new Map<string, number>();
    for (const a of baseApps) for (const t of new Set(a.tags ?? [])) m.set(t, (m.get(t) ?? 0) + 1);
    return m;
  });
  let activeTagList = $derived([...activeTags].sort());
  function matchesTags(app: CuratedApp) {
    if (activeTags.size === 0) return true;
    const t = new Set(app.tags ?? []);
    return tagMatchMode === 'all'
      ? activeTagList.every((x) => t.has(x))
      : activeTagList.some((x) => t.has(x));
  }
  let gridApps = $derived(baseApps.filter(matchesTags));

  function toggleTag(t: string) {
    const next = new Set(activeTags);
    if (next.has(t)) next.delete(t);
    else next.add(t);
    activeTags = next;
  }

  let tagMenuOpen = $state(false);
  let tagQuery = $state('');
  let tagFilterRoot = $state<HTMLDivElement | null>(null);
  let tagSearchInput = $state<HTMLInputElement | null>(null);
  let filterTrigger = $state<HTMLButtonElement | null>(null);
  let filterPos = $state<{ top: number; left: number } | null>(null);
  let matchingTagOptions = $derived(
    allTags.filter((t) => t.toLowerCase().includes(tagQuery.trim().toLowerCase()))
  );
  async function toggleTagMenu() {
    tagMenuOpen = !tagMenuOpen;
    if (!tagMenuOpen) return;
    const r = filterTrigger?.getBoundingClientRect();
    if (r) {
      const w = Math.min(330, window.innerWidth - 48);
      filterPos = { top: r.bottom + 7, left: Math.max(12, Math.min(r.right - w, window.innerWidth - w - 12)) };
    }
    tagQuery = '';
    await tick();
    tagSearchInput?.focus();
  }
  function onWindowClick(e: MouseEvent) {
    if (tagMenuOpen && !tagFilterRoot?.contains(e.target as Node)) tagMenuOpen = false;
  }
  function onWindowKey(e: KeyboardEvent) {
    if (e.key === 'Escape' && tagMenuOpen) {
      e.preventDefault();
      tagMenuOpen = false;
      filterTrigger?.focus();
    }
  }

  // Apps in the "uncategorized" bucket (added from search) can be filed away.
  let uncatKeys = $derived(
    new Set(
      (categories.find((c) => c.id === 'uncategorized')?.apps ?? []).map((a) => variantKey(a))
    )
  );
  function moveMenu(app: CuratedApp): CtxItem[] {
    if (!uncatKeys.has(variantKey(app))) return [];
    return ($curated?.categories ?? [])
      .filter((c) => c.id !== 'uncategorized')
      .map((c) => ({
        label: `Move to ${c.title || c.id}`,
        onSelect: async () => {
          if (await moveCuratedApp(app.source, app.id, c.id)) {
            await reloadCurated();
            notice(`Moved ${app.name ?? app.id} to ${c.title || c.id}.`, 'ok');
          } else {
            notice(`Couldn't move ${app.name ?? app.id}.`, 'error');
          }
        }
      }));
  }

  // --- Home ---------------------------------------------------------------
  let bodyWidth = $state(0);
  // One row per category shelf: as many cards as fit at ≥230px each.
  let shelfCols = $derived(Math.max(1, Math.floor((bodyWidth - 48) / 230)));

  async function installCollection(c: ResolvedCollection) {
    const todo = c.apps.filter((a) => !isInstalled(a));
    if (todo.length === 0) return;
    const names = todo.map((a) => a.name ?? a.id);
    const ok = await confirmAction({
      title: `Install ${todo.length} ${todo.length === 1 ? 'app' : 'apps'}?`,
      message: `${names.join(', ')}.\n\nThey install one after another; progress shows at the bottom.`,
      confirmLabel: `Install ${todo.length}`
    });
    if (ok) await installApps(todo.map(toQueued));
  }

  function queueCollection(c: ResolvedCollection) {
    const todo = c.apps.filter((a) => !isInstalled(a));
    queueApps(todo.map(toQueued));
    notice(`Added ${todo.length} ${todo.length === 1 ? 'app' : 'apps'} to the install queue.`, 'ok');
  }

  let rowLayout: 'grid' | 'list' = $derived($settings.discoverView === 'list' ? 'list' : 'grid');
  let backTo = $derived($page.url.pathname + $page.url.search);
</script>

<svelte:window onclick={onWindowClick} onkeydown={onWindowKey} onresize={() => (tagMenuOpen = false)} />

{#snippet card(app: CuratedApp, layout: 'grid' | 'list')}
  <AppCard
    name={app.name ?? app.id}
    description={app.description}
    variants={variantsOf(app)}
    installed={isInstalled(app)}
    homepage={app.icon ?? app.homepage}
    {layout}
    {backTo}
    ctxExtra={moveMenu(app)}
    onChanged={() => loadInstalled(true)}
  />
{/snippet}

{#snippet emptyState()}
  {#if noManagers}
    <div class="empty">
      <h2>No package managers found</h2>
      <p class="muted">
        Acy works on top of winget, Scoop or Chocolatey, and none were detected on this PC. Install one
        from Settings to start discovering and installing apps.
      </p>
      <a class="btn btn-accent" href="/settings" onclick={() => setSettingsTab('sources')}>Open Sources</a>
    </div>
  {:else if !$settings.showCuratedApps}
    <div class="empty">
      <h2>No apps in your list yet</h2>
      <p class="muted">
        You're only showing apps you've added. Search for an app to add it, or turn on “Show curated
        apps” in Settings → Sources.
      </p>
    </div>
  {:else if $catalog.status === 'downloading' || ($curated === null && $catalog.status !== 'error')}
    <div class="sk-grid" aria-busy="true" aria-label="Loading apps">
      {#each Array(8) as _, i (i)}
        <div class="sk-card">
          <div class="skeleton sk-icon"></div>
          <div class="sk-lines">
            <div class="skeleton sk-line lg"></div>
            <div class="skeleton sk-line sm"></div>
          </div>
        </div>
      {/each}
    </div>
  {:else}
    <div class="empty">
      <h2>Couldn't download the catalog</h2>
      <p class="muted">
        {$catalog.message || "Acy's curated apps are fetched from GitHub and haven't downloaded yet."}
      </p>
      <button class="btn" onclick={() => downloadCatalog()}>Try again</button>
    </div>
  {/if}
{/snippet}

{#snippet filterCluster()}
  {#each activeTagList as tag (tag)}
    <button class="active-tag" onclick={() => toggleTag(tag)} aria-label={`Remove ${tag} filter`} title={`Remove ${tag} filter`}>
      {tag} <X size={12} />
    </button>
  {/each}
  {#if allTags.length > 0}
    <div class="filter-wrap" bind:this={tagFilterRoot}>
      <button
        class="btn filter-trigger"
        class:on={activeTags.size > 0}
        bind:this={filterTrigger}
        onclick={toggleTagMenu}
        aria-haspopup="dialog"
        aria-expanded={tagMenuOpen}
      >
        <Funnel size={15} /> Filter
        {#if activeTags.size > 0}<span class="filter-count">{activeTags.size}</span>{/if}
      </button>
      {#if tagMenuOpen}
        <div
          class="filter-pop card"
          role="dialog"
          aria-label="Filter apps by tag"
          style="top:{filterPos?.top ?? 0}px; left:{filterPos?.left ?? 0}px"
        >
          <div class="filter-head">
            <strong>Filter by tags</strong>
            {#if activeTags.size > 0}
              <button class="link-btn" onclick={() => (activeTags = new Set())}>Clear all</button>
            {/if}
          </div>
          <input class="text-input" bind:this={tagSearchInput} bind:value={tagQuery} placeholder="Find a tag…" aria-label="Find a tag" />
          <div class="match-mode" role="group" aria-label="Tag matching mode">
            <button class:on={tagMatchMode === 'all'} aria-pressed={tagMatchMode === 'all'} onclick={() => (tagMatchMode = 'all')}>Match all</button>
            <button class:on={tagMatchMode === 'any'} aria-pressed={tagMatchMode === 'any'} onclick={() => (tagMatchMode = 'any')}>Match any</button>
          </div>
          <div class="tag-options">
            {#each matchingTagOptions as tag (tag)}
              <button class="tag-option" class:on={activeTags.has(tag)} onclick={() => toggleTag(tag)} aria-pressed={activeTags.has(tag)}>
                <span class="tag-check">{#if activeTags.has(tag)}<Check size={13} />{/if}</span>
                <span>{tag}</span>
                <span class="tag-count mono">{tagCounts.get(tag) ?? 0}</span>
              </button>
            {/each}
            {#if matchingTagOptions.length === 0}<span class="no-tags muted">No matching tags.</span>{/if}
          </div>
        </div>
      {/if}
    </div>
  {/if}
  <ViewToggle value={$settings.discoverView} onChange={setDiscoverView} />
{/snippet}

{#if view === 'home'}
  <div class="view">
    <header class="view-head">
      <div>
        <h1>Discover</h1>
        <p class="view-sub">Hand-picked apps, installed through the package managers you already use.</p>
      </div>
      {#if allApps.length > 0}
        <a class="btn" href="/?view=all">Browse all {allApps.length} apps <ArrowRight size={15} /></a>
      {/if}
    </header>
    <div class="view-body" bind:clientWidth={bodyWidth}>
      {#if categories.length === 0}
        {@render emptyState()}
      {:else}
        {#if collections.length > 0}
          <section class="block" aria-labelledby="collections-title">
            <div class="block-head"><h2 id="collections-title">Collections</h2></div>
            <div class="collections">
              {#each collections as c (c.id)}
                {@const todo = c.apps.filter((a) => !isInstalled(a))}
                <article class="collection">
                  <div class="icons" aria-hidden="true">
                    {#each c.apps.slice(0, 5) as a (variantKey(a))}
                      <AppIcon name={a.name ?? a.id} size={34} source={a.source} id={a.id} homepage={a.icon ?? a.homepage} />
                    {/each}
                  </div>
                  <h3><a href={`/?collection=${encodeURIComponent(c.id)}`}>{c.title}</a></h3>
                  {#if c.description}<p class="muted">{c.description}</p>{/if}
                  <div class="coll-foot">
                    <span class="muted coll-count">
                      {c.apps.length} apps{todo.length < c.apps.length ? ` · ${c.apps.length - todo.length} installed` : ''}
                    </span>
                    {#if todo.length > 0}
                      <button class="btn btn-accent small" onclick={() => installCollection(c)} disabled={!!$batch}>
                        Install {todo.length}
                      </button>
                    {:else}
                      <span class="all-installed"><Check size={14} /> All installed</span>
                    {/if}
                  </div>
                </article>
              {/each}
            </div>
          </section>
        {/if}

        {#each categories as cat (cat.id)}
          <section class="block" aria-labelledby={`cat-${cat.id}`}>
            <div class="block-head">
              <h2 id={`cat-${cat.id}`}>{cat.title}</h2>
              <a class="see-all" href={`/?cat=${encodeURIComponent(cat.id)}`}>
                See all {cat.apps.length} <ArrowRight size={14} />
              </a>
            </div>
            <div class="shelf" style="--cols:{shelfCols}">
              {#each cat.apps.slice(0, shelfCols) as app (variantKey(app))}
                {@render card(app, 'grid')}
              {/each}
            </div>
          </section>
        {/each}
      {/if}
    </div>
  </div>
{:else}
  <div class="view">
    <header class="view-head">
      <div>
        {#if view === 'collection'}
          <h1>{currentCollection?.title ?? 'Collection'}</h1>
          {#if currentCollection?.description}<p class="view-sub">{currentCollection.description}</p>{/if}
        {:else}
          <h1>{view === 'all' ? 'All apps' : (currentCategory?.title ?? 'Category')}</h1>
          <p class="view-sub">
            {gridApps.length === baseApps.length
              ? `${baseApps.length} apps`
              : `${gridApps.length} of ${baseApps.length} apps`}
          </p>
        {/if}
      </div>
      {#if view === 'collection' && currentCollection}
        {@const todo = currentCollection.apps.filter((a) => !isInstalled(a))}
        {#if todo.length > 0}
          <button class="btn" onclick={() => queueCollection(currentCollection!)}>Add {todo.length} to queue</button>
          <button class="btn btn-accent" onclick={() => installCollection(currentCollection!)} disabled={!!$batch}>
            Install {todo.length}
          </button>
        {/if}
        <ViewToggle value={$settings.discoverView} onChange={setDiscoverView} />
      {:else}
        {@render filterCluster()}
      {/if}
    </header>
    <div class="view-body">
      {#if categories.length === 0}
        {@render emptyState()}
      {:else if baseApps.length === 0}
        <div class="empty">
          <h2>Nothing here</h2>
          <p class="muted">This {view === 'collection' ? 'collection' : 'category'} has no apps for the package managers you have turned on.</p>
          <a class="btn" href="/">Back to Discover</a>
        </div>
      {:else if gridApps.length === 0}
        <div class="empty">
          <h2>No apps match these filters</h2>
          <p class="muted">Try fewer tags, or switch between matching all and matching any.</p>
          <button class="btn" onclick={() => (activeTags = new Set())}>Clear filters</button>
        </div>
      {:else}
        <div class={rowLayout === 'list' ? 'rows' : 'grid'}>
          {#each gridApps as app (variantKey(app))}
            {@render card(app, rowLayout)}
          {/each}
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .block {
    padding: 4px 24px 22px;
  }
  .block-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 10px;
  }
  .block-head h2 {
    font-size: var(--fs-lg);
  }
  .see-all {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: var(--fs-md);
    font-weight: 500;
  }
  .shelf {
    display: grid;
    grid-template-columns: repeat(var(--cols), minmax(0, 1fr));
    border: 1px solid var(--border);
    border-radius: var(--radius);
    overflow: hidden;
  }
  /* On a backdrop there's no panel behind the grid, so give it the card fill
     the other pages' cards use. */
  :global(html[data-backdrop]) .shelf,
  :global(html[data-backdrop]) .grid,
  :global(html[data-backdrop]) .rows {
    background: var(--surface-2);
  }
  .shelf :global(.app-card) {
    border-top: none !important;
  }
  .shelf :global(.app-card:first-child) {
    border-left: none !important;
  }
  .collections {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
    gap: 12px;
  }
  .collection {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 16px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface-2);
  }
  .collection h3 {
    font-size: var(--fs-body);
    margin-top: 6px;
  }
  .collection h3 a {
    color: var(--text);
  }
  .collection p {
    margin: 0;
    font-size: var(--fs-sm);
  }
  .icons {
    display: flex;
  }
  .icons :global(> *) {
    box-shadow: 0 0 0 2px var(--surface-2);
    border-radius: 28%;
  }
  .icons :global(> * + *) {
    margin-left: -8px;
  }
  .coll-foot {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    margin-top: auto;
    padding-top: 8px;
  }
  .coll-count {
    font-size: var(--fs-sm);
  }
  .all-installed {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: var(--fs-sm);
    color: var(--text-muted);
  }
  .all-installed :global(svg) {
    color: var(--success);
  }
  .small {
    padding: 6px 12px;
    font-size: var(--fs-md);
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
    border-bottom: 1px solid var(--border);
    margin-left: -1px;
  }
  .rows {
    display: flex;
    flex-direction: column;
    border-top: 1px solid var(--border);
  }
  .empty {
    max-width: 480px;
    margin: 40px auto;
    padding: 0 24px;
    text-align: center;
  }
  .empty h2 {
    font-size: var(--fs-lg);
  }
  .empty p {
    margin: 8px 0 18px;
    font-size: var(--fs-md);
  }
  .sk-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
    gap: 14px;
    padding: 4px 24px 24px;
  }
  .sk-card {
    display: flex;
    gap: 12px;
    padding: 14px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }
  .sk-icon {
    width: 44px;
    height: 44px;
    flex-shrink: 0;
  }
  .sk-lines {
    flex: 1;
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 8px;
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

  /* Tag filter */
  .filter-wrap {
    position: relative;
  }
  .filter-trigger {
    padding: 7px 11px;
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
    border-radius: var(--radius-sm);
    background: var(--accent-fill);
    color: var(--accent-contrast);
    font-size: var(--fs-2xs);
    font-weight: 600;
  }
  .filter-pop {
    position: fixed;
    z-index: 60;
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
    font-size: var(--fs-base);
  }
  .link-btn {
    min-height: 24px;
    padding: 2px 6px;
    border: 0;
    background: transparent;
    color: var(--accent);
    font-size: var(--fs-xs);
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
    font-size: var(--fs-xs);
  }
  .match-mode button.on {
    background: var(--surface);
    color: var(--text);
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
    font-size: var(--fs-sm);
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
    background: var(--accent-fill);
  }
  .tag-count {
    color: var(--text-muted);
    font-size: var(--fs-2xs);
  }
  .no-tags {
    padding: 12px 8px;
    font-size: var(--fs-sm);
  }
  .active-tag {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    min-height: 30px;
    padding: 3px 10px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-pill);
    background: var(--surface-2);
    color: var(--text);
    font-size: var(--fs-xs);
  }
</style>

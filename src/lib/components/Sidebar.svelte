<script lang="ts">
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import {
    Compass,
    Package as PackageIcon,
    RefreshCw,
    History,
    Settings as SettingsIcon,
    Search,
    TriangleAlert,
    X
  } from '@lucide/svelte';
  import { settings, setSettingsTab, SETTINGS_TABS } from '$lib/stores/settings';
  import { curated, loadCurated } from '$lib/stores/curated';
  import { installed, actionableUpdates } from '$lib/stores/library';
  import { updaterPhase, updaterVersion } from '$lib/stores/updater';
  import { managers } from '$lib/stores/managers';
  import { ops } from '$lib/stores/ops';
  import { searchQuery, searchManagers, recordRecentSearch } from '$lib/stores/search';
  import { browseCategories, uniqueApps } from '$lib/catalog';
  import { BUCKETS, bucketKey } from '$lib/installedGroups';
  import type { Source } from '$lib/types';

  const MANAGER_NAMES: Record<Source, string> = {
    winget: 'winget',
    scoop: 'Scoop',
    choco: 'Chocolatey',
    msstore: 'Microsoft Store'
  };

  $effect(() => {
    loadCurated();
  });

  let path = $derived($page.url.pathname);
  let params = $derived($page.url.searchParams);
  let section = $derived(
    path === '/'
      ? 'discover'
      : path.startsWith('/installed')
        ? 'installed'
        : path.startsWith('/updates')
          ? 'updates'
          : path.startsWith('/activity')
            ? 'activity'
            : ['/settings', '/curated', '/scoop-buckets', '/changelog'].some((p) => path.startsWith(p))
              ? 'settings'
              : 'other'
  );

  let categories = $derived(
    browseCategories($curated, $settings.managers, $settings.showCuratedApps)
  );
  let allCount = $derived(uniqueApps(categories).length);
  let discoverSub = $derived(
    params.get('view') === 'all' ? 'all' : (params.get('cat') ?? (params.get('collection') ? '' : 'home'))
  );

  let hidden = $derived(new Set($settings.hiddenApps));
  let visibleInstalled = $derived(
    $installed.filter((p) => !hidden.has(`${p.source}:${p.id.toLowerCase()}`))
  );
  let groups = $derived(
    BUCKETS.map((b) => ({ ...b, count: visibleInstalled.filter((p) => bucketKey(p) === b.key).length })).filter(
      (b) => b.count > 0
    )
  );
  let installedGroup = $derived(params.get('group') ?? 'all');

  let updateCount = $derived(
    $actionableUpdates.length + ($updaterPhase === 'available' && $updaterVersion ? 1 : 0)
  );
  let busy = $derived($ops.some((o) => o.state === 'running' || o.state === 'queued'));
  let issues = $derived(
    $managers.filter((m) => $settings.managers[m.source] !== false && (!m.available || m.needsSetup))
  );

  function onSearchInput() {
    const q = $searchQuery.trim();
    goto(q ? `/search?q=${encodeURIComponent(q)}` : '/search', {
      replaceState: path === '/search',
      keepFocus: true,
      noScroll: true
    });
  }

  function onSearchKey(e: KeyboardEvent & { currentTarget: HTMLInputElement }) {
    if (e.key === 'Enter') {
      const q = $searchQuery.trim();
      if (!q) return;
      if (path !== '/search') goto(`/search?q=${encodeURIComponent(q)}`, { keepFocus: true });
      searchManagers(q);
      recordRecentSearch(q);
    } else if (e.key === 'Escape') {
      if ($searchQuery) {
        searchQuery.set('');
        onSearchInput();
      } else {
        e.currentTarget.blur();
      }
    }
  }

  function clearSearch() {
    searchQuery.set('');
    onSearchInput();
    document.getElementById('global-search')?.focus();
  }

  function openSettingsTab(id: (typeof SETTINGS_TABS)[number]['id']) {
    setSettingsTab(id);
    if (path !== '/settings') goto('/settings');
  }
</script>

<aside class="sidebar">
  <div class="search" role="search" title="Search apps (Ctrl+K)">
    <Search size={16} aria-hidden="true" />
    <input
      id="global-search"
      type="search"
      placeholder="Search apps"
      aria-label="Search apps"
      autocomplete="off"
      spellcheck="false"
      bind:value={$searchQuery}
      oninput={onSearchInput}
      onkeydown={onSearchKey}
    />
    {#if $searchQuery}
      <button class="clear" onclick={clearSearch} aria-label="Clear search"><X size={14} /></button>
    {/if}
  </div>

  <nav class="nav" aria-label="Main">
    <a class="item" class:active={section === 'discover'} href="/" aria-current={section === 'discover' ? 'page' : undefined}>
      <Compass size={18} aria-hidden="true" />
      <span class="label">Discover</span>
    </a>
    {#if section === 'discover' && categories.length > 0}
      <div class="sub">
        <a class="sub-item" class:active={discoverSub === 'home'} href="/" aria-current={discoverSub === 'home' ? 'true' : undefined}>
          <span>Home</span>
        </a>
        <a class="sub-item" class:active={discoverSub === 'all'} href="/?view=all" aria-current={discoverSub === 'all' ? 'true' : undefined}>
          <span>All apps</span><span class="count mono">{allCount}</span>
        </a>
        {#each categories as cat (cat.id)}
          <a
            class="sub-item"
            class:active={discoverSub === cat.id}
            href={`/?cat=${encodeURIComponent(cat.id)}`}
            aria-current={discoverSub === cat.id ? 'true' : undefined}
          >
            <span class="truncate">{cat.title}</span><span class="count mono">{cat.apps.length}</span>
          </a>
        {/each}
      </div>
    {/if}

    <a class="item" class:active={section === 'installed'} href="/installed" aria-current={section === 'installed' ? 'page' : undefined}>
      <PackageIcon size={18} aria-hidden="true" />
      <span class="label">Installed</span>
      {#if visibleInstalled.length > 0}<span class="count mono">{visibleInstalled.length}</span>{/if}
    </a>
    {#if section === 'installed' && groups.length > 1}
      <div class="sub">
        <a class="sub-item" class:active={installedGroup === 'all'} href="/installed" aria-current={installedGroup === 'all' ? 'true' : undefined}>
          <span>All</span><span class="count mono">{visibleInstalled.length}</span>
        </a>
        {#each groups as g (g.key)}
          <a
            class="sub-item"
            class:active={installedGroup === g.key}
            href={`/installed?group=${encodeURIComponent(g.key)}`}
            aria-current={installedGroup === g.key ? 'true' : undefined}
          >
            <span class="truncate">{g.label}</span><span class="count mono">{g.count}</span>
          </a>
        {/each}
      </div>
    {/if}

    <a class="item" class:active={section === 'updates'} href="/updates" aria-current={section === 'updates' ? 'page' : undefined}>
      <RefreshCw size={18} aria-hidden="true" />
      <span class="label">Updates</span>
      {#if updateCount > 0}
        <span class="badge mono" aria-label={`${updateCount} available`}>{updateCount}</span>
      {/if}
    </a>
    <a class="item" class:active={section === 'activity'} href="/activity" aria-current={section === 'activity' ? 'page' : undefined}>
      <History size={18} aria-hidden="true" />
      <span class="label">Activity</span>
      {#if busy}<span class="busy" title="Working…" aria-label="Operations running"></span>{/if}
    </a>
  </nav>

  <div class="bottom">
    {#each issues as m (m.source)}
      <button class="item warn" onclick={() => openSettingsTab('sources')} title={`Set up ${MANAGER_NAMES[m.source]} in Settings`}>
        <TriangleAlert size={16} aria-hidden="true" />
        <span class="label truncate">{MANAGER_NAMES[m.source]} needs setup</span>
      </button>
    {/each}
    <a class="item" class:active={section === 'settings'} href="/settings" aria-current={section === 'settings' ? 'page' : undefined}>
      <SettingsIcon size={18} aria-hidden="true" />
      <span class="label">Settings</span>
    </a>
    {#if section === 'settings'}
      <div class="sub">
        {#each SETTINGS_TABS as t (t.id)}
          {@const on = path === '/settings' && $settings.settingsTab === t.id}
          <button class="sub-item" class:active={on} aria-current={on ? 'true' : undefined} onclick={() => openSettingsTab(t.id)}>
            <span>{t.label}</span>
          </button>
        {/each}
      </div>
    {/if}
  </div>
</aside>

<style>
  .sidebar {
    width: var(--sidebar-w);
    flex-shrink: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 6px 8px 10px;
    user-select: none;
  }
  .search {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 8px;
    height: 36px;
    padding: 0 8px 0 10px;
    margin-bottom: 4px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--surface);
    color: var(--text-muted);
  }
  .search:focus-within {
    border-color: var(--accent);
    box-shadow: 0 0 0 1px var(--accent);
  }
  .search input {
    flex: 1;
    min-width: 0;
    border: none;
    outline: none;
    background: transparent;
    color: var(--text);
    font: inherit;
    font-size: var(--fs-base);
  }
  .search input::-webkit-search-cancel-button {
    display: none;
  }
  .clear {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    padding: 0;
    border: none;
    border-radius: 5px;
    background: transparent;
    color: var(--text-muted);
  }
  .clear:hover {
    background: var(--surface-hover);
    color: var(--text);
  }
  .nav {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .bottom {
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
    max-height: 50%;
    overflow-y: auto;
  }
  .item {
    position: relative;
    display: flex;
    align-items: center;
    gap: 11px;
    min-height: 36px;
    padding: 0 10px;
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text);
    font-size: var(--fs-base);
    font-weight: 500;
    text-align: left;
    text-decoration: none;
  }
  .item :global(svg) {
    flex-shrink: 0;
    color: var(--text-muted);
  }
  .item:hover {
    background: var(--shell-hover);
    text-decoration: none;
  }
  .item.active {
    background: var(--shell-active);
  }
  .item.active::before {
    content: '';
    position: absolute;
    left: 0;
    top: 10px;
    bottom: 10px;
    width: 3px;
    border-radius: 2px;
    background: var(--accent);
  }
  .item.active :global(svg) {
    color: var(--accent);
  }
  .item.warn {
    font-size: var(--fs-md);
    color: var(--text-muted);
  }
  .item.warn :global(svg) {
    color: var(--warning);
  }
  .label {
    flex: 1;
    min-width: 0;
  }
  .truncate {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .count {
    flex-shrink: 0;
    font-size: var(--fs-2xs);
    color: var(--text-muted);
  }
  .badge {
    flex-shrink: 0;
    min-width: 20px;
    padding: 0 6px;
    border-radius: var(--radius-pill);
    /* Tinted rather than solid, like the page's main buttons (text >= 4.8:1). */
    background: color-mix(in srgb, var(--accent) 20%, transparent);
    color: color-mix(in srgb, var(--accent) 70%, var(--text));
    font-size: var(--fs-2xs);
    font-weight: 700;
    line-height: 1.6;
    text-align: center;
  }
  .busy {
    width: 12px;
    height: 12px;
    flex-shrink: 0;
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
  .sub {
    display: flex;
    flex-direction: column;
    gap: 1px;
    margin: 1px 0 6px 29px;
  }
  .sub-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    min-height: 30px;
    padding: 0 10px;
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-muted);
    font-size: var(--fs-md);
    text-align: left;
    text-decoration: none;
  }
  .sub-item:hover {
    background: var(--shell-hover);
    color: var(--text);
    text-decoration: none;
  }
  .sub-item.active {
    background: var(--shell-active);
    color: var(--text);
    font-weight: 500;
  }
  .sub-item.active .count {
    color: var(--accent);
  }

  /* On a backdrop the wallpaper shows through, so grey text needs more weight
     and the search box becomes glass instead of a solid block. */
  :global(html[data-backdrop]) .search {
    border-color: color-mix(in srgb, var(--text) 16%, transparent);
  }
</style>

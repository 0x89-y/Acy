<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { get } from 'svelte/store';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { Check, Funnel, Search, X } from '@lucide/svelte';
  import AppCard from '$lib/components/AppCard.svelte';
  import ViewToggle from '$lib/components/ViewToggle.svelte';
  import UpdatesSection from '$lib/components/UpdatesSection.svelte';
  import InstallButton from '$lib/components/InstallButton.svelte';
  import * as api from '$lib/api';
  import type { CuratedApp, CuratedFile, SearchHit, Source, Variant, Package } from '$lib/types';
  import { enabledSources, managers } from '$lib/stores/managers';
  import { settings, setDiscoverView, setSettingsTab, hideApp, unhideApp } from '$lib/stores/settings';
  import {
    installedKeys,
    loadInstalled,
    actionableUpdates,
    installed,
    refreshLibrary,
    isAcyPackage
  } from '$lib/stores/library';
  import { updaterPhase, updaterVersion } from '$lib/stores/updater';
  import { bucketKey, BUCKETS, BUCKET_BY_KEY, type LibSelection } from '$lib/installedGroups';
  import { runOp, summarizeBatch, installCommand } from '$lib/install';
  import { confirmAction } from '$lib/stores/confirm';
  import { copyText } from '$lib/clipboard';
  import {
    addToCurated,
    searchHitToInput,
    curatedKeys as allCuratedKeys,
    curatedKey,
    moveCuratedApp
  } from '$lib/stores/curated';
  import { notice } from '$lib/stores/ops';
  import { pendingTag, browseView } from '$lib/stores/discover';
  import type { CtxItem } from '$lib/stores/contextMenu';

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

  // ---- Discover vs Library mode (the rail switch) ----
  // Mode lives in the `browseView` store (so Ctrl+2 / the /installed redirect can
  // drive it even when this route is already mounted). Honour ?view=library once.
  if (get(page).url.searchParams.get('view') === 'library') browseView.set('library');
  // The selected Library rail entry: 'all', a bucket key, or 'updates'.
  let libSelection = $state<LibSelection>('all');
  function setMode(mode: 'discover' | 'library') {
    if (mode === $browseView) return;
    browseView.set(mode);
    exitSelect();
  }
  // Pending-update count for the Library rail badge (incl. an Acy self-update).
  let acyUpdatePending = $derived($updaterPhase === 'available' && !!$updaterVersion);
  let railUpdateCount = $derived($actionableUpdates.length + (acyUpdatePending ? 1 : 0));

  // Installed buckets as Library rail categories: every non-empty bucket, with
  // counts (hidden apps excluded). The show-scope only trims "All apps" below —
  // the individual buckets always stay reachable from the rail.
  let installedBuckets = $derived.by(() => {
    const hidden = new Set($settings.hiddenApps);
    const items = $installed.filter((p) => !hidden.has(`${p.source}:${p.id.toLowerCase()}`));
    return BUCKETS.map((b) => ({
      ...b,
      count: items.filter((p) => bucketKey(p) === b.key).length
    })).filter((b) => b.count > 0);
  });
  let installedAllCount = $derived.by(() => {
    const hidden = new Set($settings.hiddenApps);
    return $installed.filter((p) => !hidden.has(`${p.source}:${p.id.toLowerCase()}`)).length;
  });
  // Snap back to All apps if the selected bucket is filtered/emptied away.
  $effect(() => {
    if (
      libSelection !== 'all' &&
      libSelection !== 'updates' &&
      !installedBuckets.some((b) => b.key === libSelection)
    ) {
      libSelection = 'all';
    }
  });

  // Enabled managers that aren't available / still need setup — shown as rail
  // entries that jump to Settings → Sources (where they're actually managed).
  const managerNames: Record<Source, string> = {
    winget: 'winget',
    scoop: 'Scoop',
    choco: 'Chocolatey',
    msstore: 'Microsoft Store',
    local: 'Local file'
  };
  let managerIssues = $derived(
    $managers.filter(
      (m) =>
        m.source !== 'local' &&
        $settings.managers[m.source] !== false &&
        (!m.available || m.needsSetup)
    )
  );
  function openSources() {
    setSettingsTab('sources');
    goto('/settings');
  }

  // ---- Installed apps shown in the Library pane (rendered with AppCard) ----
  let libFilter = $state('');
  let showHidden = $state(false);
  let removing = $state(false);
  const hideKey = (p: Package) => `${p.source}:${p.id.toLowerCase()}`;
  let hiddenSet = $derived(new Set($settings.hiddenApps));
  let hiddenCount = $derived($installed.filter((p) => hiddenSet.has(hideKey(p))).length);
  let libQ = $derived(libFilter.trim().toLowerCase());
  let installedSorted = $derived(
    [...$installed]
      .filter(
        (p) =>
          (showHidden || !hiddenSet.has(hideKey(p))) &&
          (!libQ || p.name.toLowerCase().includes(libQ) || p.id.toLowerCase().includes(libQ))
      )
      .sort((a, b) => a.name.localeCompare(b.name))
  );
  let paneInstalled = $derived(
    libSelection === 'all' || libSelection === 'updates'
      ? installedSorted
      : installedSorted.filter((p) => bucketKey(p) === libSelection)
  );

  // Catalog description + tags so installed rows read like their Discover rows.
  let catalogInfo = $derived.by(() => {
    const desc = new Map<string, string>();
    const tags = new Map<string, string[]>();
    const k = (s: Source, i: string) => `${s}:${i.toLowerCase()}`;
    for (const cat of curated?.categories ?? []) {
      for (const app of cat.apps) {
        const set = (s: Source, i: string) => {
          if (app.description) desc.set(k(s, i), app.description);
          if (app.tags?.length) tags.set(k(s, i), app.tags);
        };
        set(app.source, app.id);
        for (const alt of app.alternates ?? []) set(alt.source, alt.id);
      }
    }
    return { desc, tags };
  });
  const descFor = (p: Package) =>
    catalogInfo.desc.get(hideKey(p)) ?? `${p.id}${p.version ? ` · ${p.version}` : ''}`;
  const tagsFor = (p: Package) => catalogInfo.tags.get(hideKey(p)) ?? [];

  // Cross-manager dupe note (managed apps only), folded into the description.
  let managedByName = $derived.by(() => {
    const m = new Map<string, Set<Source>>();
    for (const p of $installed) {
      const b = BUCKET_BY_KEY.get(bucketKey(p));
      if (!b || !b.managed) continue;
      const n = p.name.trim().toLowerCase();
      if (!n) continue;
      if (!m.has(n)) m.set(n, new Set());
      m.get(n)!.add(p.source);
    }
    return m;
  });
  function dupeSuffix(p: Package): string {
    const set = managedByName.get(p.name.trim().toLowerCase());
    if (!set || set.size < 2) return '';
    const others = [...set].filter((s) => s !== p.source);
    return others.length ? ` · also via ${others.join(', ')}` : '';
  }

  async function uninstallOne(p: Package) {
    if (isAcyPackage(p)) {
      await confirmAction({
        title: "Acy can't uninstall itself",
        message: 'To remove Acy, use Windows Settings → Apps.',
        confirmLabel: 'OK',
        alert: true
      });
      return;
    }
    const ok = await confirmAction({ title: `Remove ${p.name}?`, confirmLabel: 'Uninstall', danger: true });
    if (!ok) return;
    await runOp('uninstall', p.source, p.id, p.name);
    refreshLibrary();
  }

  function installedMenu(p: Package): CtxItem[] {
    const back = encodeURIComponent('/?view=library');
    const items: CtxItem[] = [
      { label: 'Uninstall', danger: true, onSelect: () => uninstallOne(p) },
      {
        label: 'Open details',
        onSelect: () => goto(`/app/${p.source}/${encodeURIComponent(p.id)}?back=${back}`)
      }
    ];
    const cmd = installCommand(p.source, p.id);
    if (cmd) items.push({ label: 'Copy command', onSelect: () => copyText(cmd) });
    items.push({ label: 'Copy id', onSelect: () => copyText(p.id) });
    if (p.homepage) items.push({ label: 'Open homepage', onSelect: () => openUrl(p.homepage!) });
    items.push(
      hiddenSet.has(hideKey(p))
        ? { label: 'Unhide from list', onSelect: () => unhideApp(hideKey(p)) }
        : { label: 'Hide from list', onSelect: () => hideApp(hideKey(p)) }
    );
    return items;
  }

  // Multi-select uninstall (reuses the Discover selection state/bar).
  function toggleSelectInstalled(p: Package) {
    const k = `${p.source}:${p.id}`;
    const next = new Map(selectedApps);
    if (next.has(k)) next.delete(k);
    else next.set(k, { name: p.name, variants: [{ source: p.source, id: p.id }] });
    selectedApps = next;
  }
  async function uninstallSelected() {
    removing = true;
    const total = selectedApps.size;
    let ok = 0;
    let current = 0;
    for (const entry of selectedApps.values()) {
      current++;
      installProgress = { current, total, name: entry.name };
      const v = entry.variants[0];
      if (await runOp('uninstall', v.source, v.id, entry.name)) ok++;
    }
    installProgress = null;
    removing = false;
    exitSelect();
    refreshLibrary();
    if (total > 1) summarizeBatch(total, ok, 'removed');
  }

  // Right-pane container class + per-card layout for the current view.
  let rightClass: 'grid' | 'pane-rows' = $derived(
    $settings.discoverView === 'list' ? 'pane-rows' : 'grid'
  );
  let rowLayout: 'grid' | 'list' = $derived($settings.discoverView === 'list' ? 'list' : 'grid');
  // True once the loaded manager results match the current query (i.e. the user
  // pressed Enter / Search for exactly what's in the box).
  let searchedCurrent = $derived(showSearch && searchedQuery === trimmed);

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
  let filterTrigger = $state<HTMLButtonElement | null>(null);
  // The filter popover is position:fixed (so it escapes the panel's overflow
  // clip); anchor it to the trigger button each time it opens.
  let filterPos = $state<{ top: number; left: number } | null>(null);
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
      const r = filterTrigger?.getBoundingClientRect();
      if (r) {
        const w = Math.min(330, window.innerWidth - 48);
        // Right-align to the trigger when a left-aligned menu would overflow.
        const left = Math.max(12, Math.min(r.left, window.innerWidth - w - 12));
        filterPos = { top: r.bottom + 7, left };
      } else {
        filterPos = null;
      }
      tagQuery = '';
      await tick();
      tagSearchInput?.focus();
    }
  }
  function closeTagMenu() {
    if (tagMenuOpen) tagMenuOpen = false;
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

  // ---- List view: category master-detail ----
  // The rail entry selected on the left ('all' or a category id).
  let selectedCat = $state<string>('all');
  // Snap back to "All apps" if the chosen category is filtered/removed.
  $effect(() => {
    if (selectedCat !== 'all' && !visibleCategories.some((c) => c.id === selectedCat)) {
      selectedCat = 'all';
    }
  });
  // Apps shown in the right pane, de-duped across categories for "All apps".
  let paneApps = $derived.by(() => {
    const cats =
      selectedCat === 'all'
        ? visibleCategories
        : visibleCategories.filter((c) => c.id === selectedCat);
    const seen = new Set<string>();
    const out: CuratedApp[] = [];
    for (const c of cats)
      for (const a of c.apps) {
        const k = `${a.source}:${a.id.toLowerCase()}`;
        if (!seen.has(k)) {
          seen.add(k);
          out.push(a);
        }
      }
    // "All apps" spans every category, so sort it alphabetically; a single
    // category keeps its curated order.
    if (selectedCat === 'all') {
      out.sort((a, b) => (a.name ?? a.id).localeCompare(b.name ?? b.id));
    }
    return out;
  });
  let paneTitle = $derived(
    selectedCat === 'all'
      ? 'All apps'
      : (visibleCategories.find((c) => c.id === selectedCat)?.title ?? 'All apps')
  );
  let allAppsCount = $derived(
    new Set(
      visibleCategories.flatMap((c) => c.apps.map((a) => `${a.source}:${a.id.toLowerCase()}`))
    ).size
  );
  // Apps in the "Uncategorized" bucket get the right-click "Move to…" menu.
  let uncatKeys = $derived(
    new Set(
      (visibleCategories.find((c) => c.id === 'uncategorized')?.apps ?? []).map(
        (a) => `${a.source}:${a.id}`
      )
    )
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
        const tagHit = (app.tags ?? []).some((t) => t.toLowerCase().includes(q));
        if (name.includes(q) || app.id.toLowerCase().includes(q) || tagHit) {
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

  // Which apps are already in the curated list, so search results can show
  // "In your list" instead of an Add button.
  let curatedKeySet = $derived(allCuratedKeys(curated));
  function hitInList(hit: SearchHit): boolean {
    return hit.variants.some((v) => curatedKeySet.has(curatedKey(v.source, v.id)));
  }

  async function addHit(hit: SearchHit) {
    const res = await addToCurated(searchHitToInput(hit));
    // Re-fetch so the card flips to "In your list" and the Uncategorized
    // section shows the new app.
    if (res !== 'error') curated = await api.getCurated();
    if (res === 'added') notice(`Added ${hit.name} to your list.`, 'ok');
    else if (res === 'exists') notice(`${hit.name} is already in your list.`, 'ok');
    else notice(`Couldn't add ${hit.name} to your list.`, 'error');
  }

  // A tag clicked on a card or the app page: switch to browse and show only that
  // tag's apps.
  $effect(() => {
    const t = $pendingTag;
    if (!t) return;
    browseView.set('discover');
    activeTags = new Set([t]);
    query = '';
    pendingTag.set(null);
    if (typeof window !== 'undefined') window.scrollTo({ top: 0 });
  });

  // Right-click "Move to <category>" items for an app in the Uncategorized group.
  function moveMenu(app: CuratedApp): CtxItem[] {
    return (curated?.categories ?? [])
      .filter((c) => c.id !== 'uncategorized')
      .map((c) => ({
        label: `Move to ${c.title || c.id}`,
        onSelect: () => moveApp(app, c.id, c.title || c.id)
      }));
  }
  async function moveApp(app: CuratedApp, toId: string, toTitle: string) {
    const ok = await moveCuratedApp(app.source, app.id, toId);
    if (ok) {
      curated = await api.getCurated();
      notice(`Moved ${app.name ?? app.id} to ${toTitle}.`, 'ok');
    } else {
      notice(`Couldn't move ${app.name ?? app.id}.`, 'error');
    }
  }
</script>

<svelte:window onclick={onWindowClick} onresize={closeTagMenu} onscroll={closeTagMenu} />

{#snippet selectBtn()}
  <button
    class="btn sel-toggle"
    onclick={() => (selectMode ? exitSelect() : (selectMode = true))}
    aria-pressed={selectMode}
  >
    {selectMode ? 'Cancel selection' : 'Select apps'}
  </button>
{/snippet}

{#snippet uninstallAction(app: { source: Source; id: string; name: string })}
  <InstallButton
    source={app.source}
    id={app.id}
    name={app.name}
    kind="uninstall"
    onDone={() => refreshLibrary()}
  />
{/snippet}

{#snippet filterCluster()}
  <div class="filter-wrap" bind:this={tagFilterRoot}>
    <button
      class="btn filter-trigger"
      class:on={activeTags.size > 0}
      bind:this={filterTrigger}
      onclick={toggleTagMenu}
      title="Filter by tags"
      aria-label="Filter by tags"
      aria-haspopup="dialog"
      aria-expanded={tagMenuOpen}
    >
      <Funnel size={16} />
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
{/snippet}

{#if selectMode && selectedApps.size > 0}
  <div class="sel-bar">
    <span class="sel-count" role="status" aria-live="polite">
      {#if installProgress}
        {installProgress.current} of {installProgress.total} · {installProgress.name}
      {:else}
        {selectedApps.size} selected
      {/if}
    </span>
    <div class="sel-spacer"></div>
    {#if $browseView === 'library'}
      <button class="btn btn-accent" onclick={uninstallSelected} disabled={removing}>
        {installProgress ? `${installProgress.current} of ${installProgress.total}…` : `Uninstall ${selectedApps.size}`}
      </button>
    {:else}
      <button class="btn btn-accent" onclick={installSelected} disabled={installing}>
        {installProgress ? `${installProgress.current} of ${installProgress.total}…` : `Install ${selectedApps.size}`}
      </button>
    {/if}
  </div>
{/if}

{#snippet searchResultList(cls: 'grid' | 'pane-rows')}
  {@const rowLayout = cls === 'grid' ? 'grid' : 'list'}
  {#if curatedMatches.length > 0}
    <section class="res-section">
      <h2 class="res-head">From your list</h2>
      <div class={cls}>
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
            layout={rowLayout}
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
    <p class="muted pane-msg">Searching package managers…</p>
  {:else if searchError}
    <p class="error pane-msg">{searchError}</p>
  {:else if searchedCurrent}
    {#if managerResults.length > 0}
      <section class="res-section">
        <h2 class="res-head">From package managers</h2>
        <div class={cls}>
          {#each managerResults as hit (hit.name + hit.variants[0].id)}
            <AppCard
              name={hit.name}
              description={hit.description}
              variants={hit.variants.map((v) => ({ source: v.source, id: v.id }))}
              installed={hitInstalled(hit)}
              layout={rowLayout}
              highlight={trimmed}
              inList={hitInList(hit)}
              onAddToList={() => addHit(hit)}
              onChanged={() => loadInstalled(true)}
            />
          {/each}
        </div>
      </section>
    {:else}
      <p class="muted pane-msg">
        {curatedMatches.length === 0
          ? `No results for “${query}”.`
          : `No package-manager results for “${query}”.`}
      </p>
    {/if}
  {:else if curatedMatches.length === 0}
    <p class="muted pane-msg">No curated apps match “{query}”.</p>
  {/if}
{/snippet}

{#if noManagers}
  <div class="empty card">
    <h2>No package managers found</h2>
    <p class="muted">
      Acy works on top of winget, Scoop, or Chocolatey, and none were detected on this machine.
      Install one from Settings to start discovering and installing apps.
    </p>
    <a class="btn btn-accent" href="/settings">Open Settings</a>
  </div>
{:else}
  <!-- One master-detail panel. The rail switches between Discover (category
       browse) and Library (Installed / Updates); the right pane follows. The
       rail hides during a Discover search so results fill the pane. -->
  <div class="browse-panel" bind:this={resultsEl}>
    {#if !showSearch}
      <div class="browse-rail">
        <div class="rail-switch">
          <button class:on={$browseView === 'discover'} onclick={() => setMode('discover')}>Discover</button>
          <button class:on={$browseView === 'library'} onclick={() => setMode('library')}>
            Library
            {#if railUpdateCount > 0 || managerIssues.length > 0}
              <span
                class="lib-dot"
                class:warn={managerIssues.length > 0}
                title={managerIssues.length > 0 ? 'A source needs attention' : 'Updates available'}
              ></span>
            {/if}
          </button>
        </div>
        {#if $browseView === 'discover'}
          <button
            class="rail-link"
            class:active={selectedCat === 'all'}
            onclick={() => (selectedCat = 'all')}
          >
            <span>All apps</span><span class="rail-count mono">{allAppsCount}</span>
          </button>
          {#each visibleCategories as cat (cat.id)}
            <button
              class="rail-link"
              class:active={selectedCat === cat.id}
              onclick={() => (selectedCat = cat.id)}
            >
              <span>{cat.title}</span><span class="rail-count mono">{cat.apps.length}</span>
            </button>
          {/each}
        {:else}
          <button
            class="rail-link"
            class:active={libSelection === 'all'}
            onclick={() => (libSelection = 'all')}
          >
            <span>All apps</span><span class="rail-count mono">{installedAllCount}</span>
          </button>
          {#each installedBuckets as b (b.key)}
            <button
              class="rail-link"
              class:active={libSelection === b.key}
              onclick={() => (libSelection = b.key)}
            >
              <span>{b.label}</span><span class="rail-count mono">{b.count}</span>
            </button>
          {/each}
          {#each managerIssues as m (m.source)}
            <button
              class="rail-link setup-link"
              onclick={openSources}
              title={`Set up ${managerNames[m.source]} in Settings`}
            >
              <span class="setup-name">
                <span class="dot" class:warn={m.available} class:off={!m.available}></span>
                {managerNames[m.source]}
              </span>
              <span class="rail-warn mono">set up</span>
            </button>
          {/each}
          <button
            class="rail-link updates-link"
            class:active={libSelection === 'updates'}
            onclick={() => (libSelection = 'updates')}
          >
            <span>Updates</span>
            {#if railUpdateCount > 0}<span class="rail-badge mono">{railUpdateCount}</span>{/if}
          </button>
        {/if}
      </div>
    {/if}
    <div class="browse-main">
      {#if $browseView === 'library'}
        {#if libSelection === 'updates'}
          <UpdatesSection />
        {:else}
          <div class="pane-head">
            <input
              class="lib-filter"
              placeholder="Filter apps…"
              aria-label="Filter apps"
              bind:value={libFilter}
            />
            {@render selectBtn()}
            {#if hiddenCount > 0}
              <button
                class="btn btn-ghost"
                onclick={() => (showHidden = !showHidden)}
                aria-pressed={showHidden}
              >
                {showHidden ? 'Hide hidden' : `Show hidden (${hiddenCount})`}
              </button>
            {/if}
            <div class="pane-tools">
              <ViewToggle value={$settings.discoverView} onChange={setDiscoverView} />
            </div>
          </div>
          <div class="pane-scroll">
            {#if paneInstalled.length === 0}
              <p class="pane-msg muted">
                {libFilter ? `No installed apps match “${libFilter}”.` : 'No apps in this group.'}
              </p>
            {:else}
              <div class={rightClass}>
                {#each paneInstalled as p (p.source + p.id)}
                  <AppCard
                    name={p.name}
                    description={descFor(p) + dupeSuffix(p)}
                    variants={[{ source: p.source, id: p.id }]}
                    homepage={p.homepage}
                    tags={tagsFor(p)}
                    layout={rowLayout}
                    backTo="/?view=library"
                    menu={installedMenu(p)}
                    selectable={selectMode}
                    selected={selectedApps.has(`${p.source}:${p.id}`)}
                    onToggleSelect={() => toggleSelectInstalled(p)}
                    action={uninstallAction}
                  />
                {/each}
              </div>
            {/if}
          </div>
        {/if}
      {:else}
        <div class="pane-head">
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
          <div class="pane-tools">
            {#if !showSearch && allTags.length > 0}{@render filterCluster()}{/if}
            {@render selectBtn()}
            <ViewToggle value={$settings.discoverView} onChange={setDiscoverView} />
          </div>
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
        <div class="pane-scroll">
          {#if loadingCurated}
            <div class="sk-grid">
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
          {:else if showSearch}
            {@render searchResultList(rightClass)}
          {:else if visibleCategories.length === 0 && activeTags.size > 0}
            <div class="filter-empty">
              <h2>No apps match these filters</h2>
              <p class="muted">Try fewer tags or switch between matching all and matching any.</p>
              <button class="btn" onclick={clearTags}>Clear filters</button>
            </div>
          {:else}
            <div class={rightClass}>
              {#each paneApps as app (app.source + app.id)}
                {@const vs = curatedVariants(app, $settings.managers)}
                <AppCard
                  name={app.name ?? app.id}
                  description={app.description}
                  variants={vs}
                  installed={anyInstalled(vs)}
                  homepage={app.icon ?? app.homepage}
                  tags={app.tags ?? []}
                  allowPick
                  layout={rowLayout}
                  selectable={selectMode && !anyInstalled(vs)}
                  selected={selectedApps.has(appKey(app))}
                  ctxExtra={uncatKeys.has(appKey(app)) ? moveMenu(app) : []}
                  onToggleSelect={() => toggleSelectApp(app)}
                  onChanged={() => loadInstalled(true)}
                />
              {/each}
            </div>
          {/if}
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .search-box {
    display: flex;
    align-items: center;
    gap: 10px;
    flex: 1;
    min-width: 180px;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    padding: 0 12px;
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
    padding: 8px 0;
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
  /* Library filter input — sits in the pane-head like Discover's search. */
  .lib-filter {
    flex: 1;
    min-width: 180px;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    color: var(--text);
    padding: 8px 12px;
    font-size: 0.9rem;
    outline: none;
  }
  .lib-filter:focus {
    border-color: var(--accent);
  }
  .recents {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
    padding: 8px 14px;
    border-bottom: 1px solid var(--border);
  }
  .recents-label {
    font-size: 0.78rem;
  }
  .recent {
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--text);
    padding: 4px 12px;
    border-radius: var(--radius-sm);
    font-size: 0.82rem;
  }
  .recent:hover {
    background: var(--surface-hover);
    border-color: var(--border-strong);
  }
  .filter-wrap {
    position: relative;
    flex-shrink: 0;
  }
  .filter-trigger {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 7px 9px;
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
    font-size: 0.7rem;
    font-weight: 600;
  }
  .filter-pop {
    position: fixed;
    z-index: 60;
    width: min(330px, calc(100vw - 48px));
    padding: 14px;
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
    background: var(--accent-fill);
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
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface-2);
    color: var(--text);
    font-size: 0.76rem;
  }
  .active-more {
    min-width: 32px;
    justify-content: center;
  }
  /* Master-detail panel (both views): category rail + apps pane. Capped to the
     viewport so the rail stays put and the apps pane scrolls. */
  .browse-panel {
    display: flex;
    align-items: stretch;
    flex: 1;
    min-height: 0;
    overflow: hidden;
    background: var(--surface);
  }
  .browse-rail {
    flex: 0 0 190px;
    display: flex;
    flex-direction: column;
    overflow-y: auto;
    border-right: 1px solid var(--border);
    background: var(--surface-2);
  }
  /* Discover | Library mode switch, pinned at the top of the rail. */
  .rail-switch {
    display: flex;
    gap: 2px;
    flex-shrink: 0;
    margin: 8px;
    padding: 2px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
  }
  .rail-switch button {
    flex: 1;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 5px;
    padding: 5px 8px;
    border: 0;
    border-radius: 5px;
    background: transparent;
    color: var(--text-muted);
    font-size: 0.82rem;
    font-weight: 500;
  }
  /* Attention dot on Library: accent = updates, amber = a source needs setup. */
  .lib-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    flex-shrink: 0;
    background: var(--accent);
  }
  .lib-dot.warn {
    background: var(--warning);
  }
  .rail-switch button.on .lib-dot {
    background: var(--accent-contrast);
  }
  .rail-switch button.on {
    background: var(--accent-fill);
    color: var(--accent-contrast);
  }
  .rail-badge {
    font-family: var(--font-mono);
    font-size: 0.7rem;
    background: var(--accent-fill);
    color: var(--accent-contrast);
    border-radius: var(--radius-sm);
    padding: 0 6px;
    line-height: 1.5;
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
  /* Push Updates to the bottom of the rail, away from the buckets. */
  .updates-link {
    margin-top: auto;
  }
  /* Managers that still need installing — muted rail entries with a status dot. */
  .setup-name {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .rail-warn {
    font-size: 0.66rem;
    color: var(--text-muted);
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
  }
  .dot.warn {
    background: var(--warning);
  }
  .dot.off {
    background: var(--text-muted);
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
  /* Pinned toolbar: category title + count on the left, filter/select right. */
  .pane-head {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 10px;
    min-height: 34px;
    padding: 10px 14px;
    border-bottom: 1px solid var(--border);
  }
  .pane-tools {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 8px;
  }
  .pane-scroll {
    flex: 1;
    min-height: 0;
    overflow: hidden auto;
  }
  .sk-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
    gap: 14px;
    padding: 14px;
  }
  .pane-rows {
    display: flex;
    flex-direction: column;
  }
  /* Grid becomes a divided grid: flush cells, hairlines between them. The -1px
     pulls the outermost top/left hairlines under the panel + pane-head frame. */
  .pane-scroll .grid {
    gap: 0;
    margin: -1px 0 0 -1px;
  }
  .browse-main .res-section {
    margin-bottom: 0;
  }
  .browse-main .res-head {
    margin-bottom: 0;
    padding: 14px 14px 10px;
  }
  .pane-msg {
    padding: 16px 14px;
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
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 14px;
    background: var(--surface);
    border-bottom: 1px solid var(--border);
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
  .res-section {
    margin-bottom: 24px;
  }
  .res-head {
    font-size: 1.05rem;
    font-weight: 600;
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
  @media (max-width: 720px) {
    .browse-panel {
      flex-direction: column;
      height: auto;
    }
    .browse-rail {
      flex: none;
      flex-direction: row;
      flex-wrap: wrap;
      overflow: visible;
      border-right: none;
      border-bottom: 1px solid var(--border);
    }
    .rail-link {
      flex: 1 1 auto;
      justify-content: center;
      border-top: none;
      border-left: none;
    }
  }
</style>

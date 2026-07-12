<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { getVersion } from '@tauri-apps/api/app';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import type { Source, Package } from '$lib/types';
  import { runOp, summarizeBatch, installCommand } from '$lib/install';
  import { copyText } from '$lib/clipboard';
  import { openContextMenu } from '$lib/stores/contextMenu';
  import { confirmAction } from '$lib/stores/confirm';
  import {
    settings,
    setInstalledSort,
    setInstalledGroup,
    setInstalledShow,
    setGroupCollapsed,
    hideApp,
    unhideApp,
    setInstalledView
  } from '$lib/stores/settings';
  import type { InstalledShow } from '$lib/stores/settings';
  import {
    installed,
    installedLoading,
    updatesLoading,
    installedError,
    updatesError,
    installedReady,
    actionableUpdates,
    ignoredUpdates,
    lastChecked,
    loadInstalled,
    loadUpdates,
    refreshLibrary,
    isAcyPackage
  } from '$lib/stores/library';
  import { ignoreUpdate, restoreUpdate } from '$lib/stores/ignoredUpdates';
  import { updaterPhase, updaterVersion, installUpdate } from '$lib/stores/updater';
  import { curated, loadCurated } from '$lib/stores/curated';
  import { Check, EyeOff, RotateCcw, ChevronDown, FileText } from '@lucide/svelte';
  import InstallButton from '$lib/components/InstallButton.svelte';
  import ConfirmAction from '$lib/components/ConfirmAction.svelte';
  import AppIcon from '$lib/components/AppIcon.svelte';
  import SourceBadge from '$lib/components/SourceBadge.svelte';
  import ViewToggle from '$lib/components/ViewToggle.svelte';

  let updatingAll = $state<Source | null>(null);
  let updatingEverything = $state(false);

  // Filter is transient; sort + grouping persist via settings.
  let filter = $state('');

  let loading = $derived($installedLoading || $updatesLoading);
  let error = $derived($installedError ?? $updatesError);
  let updateSources = $derived([...new Set($actionableUpdates.map((u) => u.source))]);
  let batchProgress = $state<{ current: number; total: number; label: string } | null>(null);
  let showIgnored = $state(false);
  let acyVersion = $state('');
  let acyUpdateAvailable = $derived($updaterPhase === 'available' && !!$updaterVersion);
  let totalUpdateCount = $derived($actionableUpdates.length + (acyUpdateAvailable ? 1 : 0));

  // Release-notes links from the curated catalog, keyed by every manager id an
  // app can be installed from, so an update row can link to its changelog.
  let releaseNotesMap = $derived.by(() => {
    const m = new Map<string, string>();
    const file = $curated;
    if (!file) return m;
    const k = (s: Source, i: string) => `${s}:${i.toLowerCase()}`;
    for (const cat of file.categories) {
      for (const app of cat.apps) {
        if (!app.releaseNotes) continue;
        m.set(k(app.source, app.id), app.releaseNotes);
        for (const alt of app.alternates ?? []) m.set(k(alt.source, alt.id), app.releaseNotes);
      }
    }
    return m;
  });
  const releaseNotesFor = (p: Package) => releaseNotesMap.get(`${p.source}:${p.id.toLowerCase()}`) ?? null;

  const sourceOrder: Source[] = ['winget', 'scoop', 'choco', 'msstore'];
  let q = $derived(filter.trim().toLowerCase());
  let acyMatches = $derived(!q || 'acy'.includes(q) || 'acy application'.includes(q));

  // Apps the user has manually hidden (persisted). Temporarily revealed via the
  // "Show hidden" toggle so they can be un-hidden.
  const hideKey = (p: Package) => `${p.source}:${p.id.toLowerCase()}`;
  let hiddenSet = $derived(new Set($settings.hiddenApps));
  let showHidden = $state(false);
  let hiddenCount = $derived($installed.filter((p) => hiddenSet.has(hideKey(p))).length);

  let filtered = $derived(
    $installed.filter(
      (p) =>
        (showHidden || !hiddenSet.has(hideKey(p))) &&
        (!q || p.name.toLowerCase().includes(q) || p.id.toLowerCase().includes(q))
    )
  );
  let sorted = $derived(
    [...filtered].sort((a, b) => {
      if ($settings.installedSort === 'source' && a.source !== b.source) {
        return sourceOrder.indexOf(a.source) - sourceOrder.indexOf(b.source);
      }
      return a.name.localeCompare(b.name);
    })
  );
  // winget surfaces lots of entries that aren't from a real package manager:
  // Add/Remove-Programs (`ARP\…`) and MSIX/AppX (`MSIX\…`) packages. We only
  // reclassify THOSE into noise buckets — real managed apps (a proper winget/
  // scoop/choco/msstore id) always stay in their manager group, so e.g. the
  // "Epic Games Launcher" managed by winget isn't yanked into Games.
  //
  // Detection is best-effort (name/id heuristics) and easy to tune.
  const GAMES =
    /(\\steam app\b|steam app|epic games|\bgog\b|gog galaxy|gog\.com|\bea app\b|ea desktop|origin games|ubisoft|uplay|battle\.net|blizzard|battlefield|\briot\b|riot games|riot client|valorant|league of legends|hoyoplay|genshin|honkai|zenless zone|vintage story)/i;
  const MS_SYSTEM =
    /(visual c\+\+|redistributable|webview2|windows sdk|windows software development kit|\.net\s+(runtime|sdk|host|desktop runtime|targeting pack)|microsoft edge update|windows app runtime)/i;
  // Microsoft's MSIX publisher hash — first-party Store/built-in packages.
  const MS_MSIX_PUBLISHER = '_8wekyb3d8bbwe';
  // Reliable signals from the ARP registry (publisher / install path), which
  // catch launcher-installed games whose name carries no marker (e.g. Diablo).
  const GAME_PUBLISHERS =
    /(valve|blizzard|ubisoft|electronic arts|rockstar|bethesda|riot games|cd projekt|epic games|\bgog\b|square enix|activision|\bsega\b|capcom|bandai namco|2k games|xbox game studios|mojang|devolver|paradox interactive|larian|mihoyo|hoyoverse|cognosphere|anego studios)/i;
  const GAME_PATHS =
    /(steamapps|[\\/]steam[\\/]|epic games|gog galaxy|gog games|[\\/]gog[\\/]|ubisoft|uplay|riot games|battle\.net|[\\/]ea games[\\/]|origin games|[\\/]games[\\/])/i;

  type BucketKey = Source | 'games' | 'ms-system' | 'other';
  function bucketKey(p: Package): BucketKey {
    if (p.source !== 'winget') return p.source;
    const idl = p.id.toLowerCase();
    const isArp = idl.startsWith('arp\\');
    const isMsix = idl.startsWith('msix\\');
    if (!isArp && !isMsix) return 'winget'; // a real winget-managed app
    const hay = `${p.name} ${p.id}`;
    const pub = (p.publisher ?? '').toLowerCase();
    const loc = (p.installLocation ?? '').toLowerCase();
    if (GAMES.test(hay) || GAME_PUBLISHERS.test(pub) || GAME_PATHS.test(loc)) return 'games';
    if (MS_SYSTEM.test(p.name)) return 'ms-system';
    if (isMsix && idl.includes(MS_MSIX_PUBLISHER)) return 'ms-system';
    return 'other';
  }

  // `system: true` buckets are hidden in the "Hide system" show mode; only
  // buckets with a `badge` (a real manager) survive "Managed only".
  type Bucket = { key: BucketKey; label: string | null; badge: Source | null; system: boolean };
  const BUCKETS: Bucket[] = [
    { key: 'winget', label: null, badge: 'winget', system: false },
    { key: 'scoop', label: null, badge: 'scoop', system: false },
    { key: 'choco', label: null, badge: 'choco', system: false },
    { key: 'msstore', label: null, badge: 'msstore', system: false },
    { key: 'games', label: 'Games', badge: null, system: false },
    { key: 'ms-system', label: 'Windows components', badge: null, system: true },
    { key: 'other', label: 'Other apps', badge: null, system: true }
  ];

  // All groups (managers included) are collapsible; collapse state persists.
  let collapsedGroups = $derived(new Set($settings.collapsedGroups));
  function toggleGroup(key: BucketKey) {
    setGroupCollapsed(key, !collapsedGroups.has(key));
  }

  let grouped = $derived(
    BUCKETS.map((b) => ({ ...b, items: sorted.filter((p) => bucketKey(p) === b.key) })).filter(
      (g) => g.items.length > 0
    )
  );

  // Apply the show-scope (all / hide system noise / manager-managed only).
  let visibleGroups = $derived(
    grouped.filter((g) => {
      if ($settings.installedShow === 'managed') return g.badge !== null;
      if ($settings.installedShow === 'hide-system') return !g.system;
      return true;
    })
  );

  // "N managed · M total" — managed = apps from a real package manager.
  let managedCount = $derived(
    grouped.filter((g) => g.badge !== null).reduce((n, g) => n + g.items.length, 0)
  );
  let totalCount = $derived(grouped.reduce((n, g) => n + g.items.length, 0));

  // Same show-scope test for the flat (un-grouped) list.
  const BUCKET_BY_KEY = new Map(BUCKETS.map((b) => [b.key, b]));
  function bucketVisible(key: BucketKey): boolean {
    const b = BUCKET_BY_KEY.get(key);
    if ($settings.installedShow === 'managed') return !!b && b.badge !== null;
    if ($settings.installedShow === 'hide-system') return !!b && !b.system;
    return true;
  }
  let visibleSorted = $derived(sorted.filter((p) => bucketVisible(bucketKey(p))));

  // Cross-manager dedupe: flag apps that appear under more than one real
  // manager (e.g. the same tool via winget AND scoop) so the duplicate is
  // visible. Matched conservatively by normalized display name, managed apps
  // only (pass-through/ARP noise is excluded to avoid false collisions).
  let managedByName = $derived.by(() => {
    const m = new Map<string, Set<Source>>();
    for (const p of $installed) {
      const b = BUCKET_BY_KEY.get(bucketKey(p));
      if (!b || b.badge === null) continue;
      const n = p.name.trim().toLowerCase();
      if (!n) continue;
      if (!m.has(n)) m.set(n, new Set());
      m.get(n)!.add(p.source);
    }
    return m;
  });
  function dupeNote(p: Package): string | null {
    const set = managedByName.get(p.name.trim().toLowerCase());
    if (!set || set.size < 2) return null;
    const others = [...set].filter((src) => src !== p.source);
    return others.length ? others.join(', ') : null;
  }

  // A coarse clock so "checked … ago" stays roughly current without re-rendering
  // constantly.
  let now = $state(Date.now());

  // Cache-aware: instant on repeat visits, fetches only the first time.
  onMount(() => {
    loadInstalled();
    loadUpdates();
    loadCurated();
    getVersion().then((version) => (acyVersion = version)).catch(() => (acyVersion = ''));
    const tick = setInterval(() => (now = Date.now()), 30_000);
    return () => clearInterval(tick);
  });

  function ago(at: number, ref: number): string {
    const secs = Math.max(0, Math.round((ref - at) / 1000));
    if (secs < 60) return 'just now';
    const mins = Math.round(secs / 60);
    if (mins < 60) return `${mins} min ago`;
    const hrs = Math.round(mins / 60);
    if (hrs < 24) return `${hrs} hr ago`;
    return `${Math.round(hrs / 24)} d ago`;
  }

  async function updateAll(source: Source) {
    const targets = $actionableUpdates.filter((u) => u.source === source);
    if (targets.length === 0) return;
    updatingAll = source;
    let ok = 0;
    for (const [i, p] of targets.entries()) {
      batchProgress = { current: i + 1, total: targets.length, label: `Updating ${p.name}` };
      if (await runOp('update', p.source, p.id, p.name)) ok++;
    }
    batchProgress = null;
    updatingAll = null;
    await refreshLibrary();
    if (targets.length > 1) summarizeBatch(targets.length, ok, 'updated');
  }

  // Update every manager that has pending updates, one after another.
  async function updateEverything() {
    updatingEverything = true;
    for (const s of updateSources) {
      await updateAll(s);
    }
    if (acyUpdateAvailable) await installUpdate();
    updatingEverything = false;
  }

  // ---- Multi-select uninstall ----
  function selKey(p: Package) {
    return `${p.source}:${p.id}`;
  }
  let selected = $state<Set<string>>(new Set());
  let selectMode = $state(false);
  let selectedCount = $derived(selected.size);

  function toggleSel(k: string) {
    const next = new Set(selected);
    if (next.has(k)) next.delete(k);
    else next.add(k);
    selected = next;
  }
  function clearSel() {
    selected = new Set();
  }

  function exitSelect() {
    clearSel();
    selectMode = false;
  }

  // Batch uninstall: one confirm for the whole selection, then run sequentially.
  let removing = $state(false);

  async function uninstallSelected() {
    removing = true;
    const targets = $installed.filter((p) => selected.has(selKey(p)));
    let ok = 0;
    for (const [i, p] of targets.entries()) {
      batchProgress = { current: i + 1, total: targets.length, label: `Removing ${p.name}` };
      if (await runOp('uninstall', p.source, p.id, p.name)) ok++;
    }
    batchProgress = null;
    removing = false;
    exitSelect();
    await refreshLibrary();
    if (targets.length > 1) summarizeBatch(targets.length, ok, 'removed');
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
    const ok = await confirmAction({
      title: `Remove ${p.name}?`,
      confirmLabel: 'Uninstall',
      danger: true
    });
    if (!ok) return;
    await runOp('uninstall', p.source, p.id, p.name);
    refreshLibrary();
  }

  function rowMenu(e: MouseEvent, p: Package) {
    openContextMenu(e, [
      { label: 'Uninstall', danger: true, onSelect: () => uninstallOne(p) },
      {
        label: 'Open details',
        onSelect: () => goto(`/app/${p.source}/${encodeURIComponent(p.id)}?from=installed`)
      },
      ...(installCommand(p.source, p.id)
        ? [{ label: 'Copy command', onSelect: () => copyText(installCommand(p.source, p.id)!) }]
        : []),
      { label: 'Copy id', onSelect: () => copyText(p.id) },
      ...(p.homepage ? [{ label: 'Open homepage', onSelect: () => openUrl(p.homepage!) }] : []),
      hiddenSet.has(hideKey(p))
        ? { label: 'Unhide from list', onSelect: () => unhideApp(hideKey(p)) }
        : { label: 'Hide from list', onSelect: () => hideApp(hideKey(p)) }
    ]);
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && selectMode) exitSelect();
  }

</script>

<svelte:window onkeydown={onKeydown} />

{#snippet acyRow()}
  <div class="row card acy-row" role="group">
    <img class="acy-icon" src="/acy-icon.png" alt="" />
    <div class="info">
      <span class="name">Acy</span>
      <div class="ver mono">acy{acyVersion ? ` · ${acyVersion}` : ''}</div>
    </div>
    <span class="acy-badge">Acy</span>
    <div class="row-action"><span class="current-app">Current app</span></div>
  </div>
{/snippet}

<div class="head">
  <h1>Installed</h1>
  {#if $lastChecked}
    <span class="checked muted">Checked {ago($lastChecked, now)}</span>
  {/if}
  <button class="btn btn-ghost" onclick={() => refreshLibrary()} disabled={loading}>
    {loading ? 'Refreshing…' : 'Refresh'}
  </button>
</div>

{#if loading && !$installedReady}
  <div class="list">
    {#each Array(6) as _, i (i)}
      <div class="row card">
        <div class="skeleton sk-icon"></div>
        <div class="sk-lines">
          <div class="skeleton sk-line lg"></div>
          <div class="skeleton sk-line sm"></div>
        </div>
      </div>
    {/each}
  </div>
{:else if error && $installed.length === 0}
  <p class="error">{error}</p>
{:else}
  {#if error}
    <p class="warn-banner">{error} Showing the last known list — try Refresh again.</p>
  {/if}
  {#if totalUpdateCount > 0}
    <section class="block">
      <div class="block-head">
        <h2>Updates <span class="count">{totalUpdateCount}</span></h2>
        <div class="all-actions">
          {#if totalUpdateCount > 1}
            <button
              class="btn btn-accent"
              onclick={updateEverything}
              disabled={updatingEverything || updatingAll !== null || $updaterPhase === 'downloading'}
            >
              {updatingEverything ? 'Updating…' : `Update everything · ${totalUpdateCount}`}
            </button>
          {/if}
          {#each updateSources as s (s)}
            <button
              class="btn"
              onclick={() => updateAll(s)}
              disabled={updatingAll === s || updatingEverything}
            >
              {updatingAll === s ? 'Updating…' : `Update all · ${s}`}
            </button>
          {/each}
        </div>
      </div>
      {#if batchProgress}
        <div class="batch-progress" role="status" aria-live="polite">
          <span>{batchProgress.label}</span>
          <span class="mono">{batchProgress.current} of {batchProgress.total}</span>
        </div>
      {/if}
      <div class="list">
        {#if acyUpdateAvailable}
          <div class="row card">
            <img class="acy-icon" src="/acy-icon.png" alt="" />
            <div class="info">
              <span class="name">Acy</span>
              <div class="ver mono">{acyVersion || '?'} → {$updaterVersion}</div>
            </div>
            <span class="acy-badge">Acy</span>
            <button
              class="btn btn-accent"
              onclick={installUpdate}
              disabled={$updaterPhase === 'downloading'}
            >
              {$updaterPhase === 'downloading' ? 'Installing…' : 'Update Acy'}
            </button>
          </div>
        {/if}
        {#each $actionableUpdates as p (p.source + p.id)}
          <div class="row card">
            <AppIcon name={p.name} size={36} source={p.source} id={p.id} homepage={p.homepage} />
            <div class="info">
              <a class="name" href={`/app/${p.source}/${encodeURIComponent(p.id)}?from=installed`}>{p.name}</a>
              <div class="ver mono">{p.version ?? '?'} → {p.availableVersion ?? '?'}</div>
            </div>
            <SourceBadge source={p.source} />
            {#if releaseNotesFor(p)}
              <button
                class="icon-action"
                onclick={() => openUrl(releaseNotesFor(p)!)}
                title="Release notes"
                aria-label={`Release notes for ${p.name}`}
              >
                <FileText size={16} />
              </button>
            {/if}
            <button
              class="icon-action"
              onclick={() => ignoreUpdate(p)}
              title="Ignore this version"
              aria-label={`Ignore ${p.name} version ${p.availableVersion ?? ''}`}
            >
              <EyeOff size={16} />
            </button>
            <InstallButton
              source={p.source}
              id={p.id}
              name={p.name}
              kind="update"
              onDone={refreshLibrary}
            />
          </div>
        {/each}
      </div>
    </section>
  {/if}

  {#if totalUpdateCount === 0}
    <p class="uptodate">
      {$ignoredUpdates.length > 0 ? 'No active updates.' : '✓ Everything is up to date.'}
    </p>
  {/if}

  {#if $ignoredUpdates.length > 0}
    <section class="ignored-block">
      <button class="ignored-toggle" onclick={() => (showIgnored = !showIgnored)} aria-expanded={showIgnored}>
        Ignored updates <span class="count soft">{$ignoredUpdates.length}</span>
      </button>
      {#if showIgnored}
        <div class="list ignored-list">
          {#each $ignoredUpdates as p (p.source + p.id)}
            <div class="row card">
              <AppIcon name={p.name} size={36} source={p.source} id={p.id} homepage={p.homepage} />
              <div class="info">
                <span class="name">{p.name}</span>
                <div class="ver mono">{p.version ?? '?'} → {p.availableVersion ?? '?'}</div>
              </div>
              <SourceBadge source={p.source} />
              <button class="btn btn-ghost" onclick={() => restoreUpdate(p)}>
                <RotateCcw size={14} /> Restore
              </button>
            </div>
          {/each}
        </div>
      {/if}
    </section>
  {/if}

  <section class="block">
    <div class="block-head">
      <h2>
        All installed
        <span class="count soft">{managedCount} managed · {totalCount} total</span>
      </h2>
    </div>
    {#if $installed.length === 0 && !acyMatches}
      <p class="muted">No installed apps match “{filter}”.</p>
    {:else}
      {#if selectedCount > 0}
        <div class="sel-bar">
          <span class="sel-count">{selectedCount} selected</span>
          <div class="sel-spacer"></div>
          <button class="btn btn-ghost" onclick={clearSel} disabled={removing}>Clear</button>
          <ConfirmAction
            label={`Uninstall ${selectedCount}`}
            message={`Remove ${selectedCount} selected ${selectedCount === 1 ? 'app' : 'apps'}?`}
            confirmLabel="Uninstall"
            busyLabel={batchProgress ? `${batchProgress.current} of ${batchProgress.total}…` : 'Removing…'}
            busy={removing}
            onConfirm={uninstallSelected}
          />
        </div>
      {/if}
      <div class="toolbar">
        <input class="filter" placeholder="Filter installed…" aria-label="Filter installed apps" bind:value={filter} />
        <button
          class="btn"
          onclick={() => (selectMode ? exitSelect() : (selectMode = true))}
          aria-pressed={selectMode}
        >
          {selectMode ? 'Cancel selection' : 'Select apps'}
        </button>
        <label class="sort">
          Sort
          <select
            value={$settings.installedSort}
            onchange={(e) => setInstalledSort(e.currentTarget.value as 'name' | 'source')}
          >
            <option value="name">Name</option>
            <option value="source">Manager</option>
          </select>
        </label>
        <label class="sort">
          Show
          <select
            value={$settings.installedShow}
            onchange={(e) => setInstalledShow(e.currentTarget.value as InstalledShow)}
          >
            <option value="all">All</option>
            <option value="hide-system">Hide system</option>
            <option value="managed">Managed only</option>
          </select>
        </label>
        <label class="group-toggle">
          Grouped
          <span class="switch">
            <input
              type="checkbox"
              checked={$settings.installedGroup}
              onchange={(e) => setInstalledGroup(e.currentTarget.checked)}
            />
            <span class="slider"></span>
          </span>
        </label>
        {#if hiddenCount > 0}
          <button class="btn btn-ghost" onclick={() => (showHidden = !showHidden)} aria-pressed={showHidden}>
            {showHidden ? 'Hide hidden' : `Show hidden (${hiddenCount})`}
          </button>
        {/if}
        <div class="tb-spacer"></div>
        <ViewToggle value={$settings.installedView} onChange={setInstalledView} />
      </div>

      {#if visibleSorted.length === 0 && !acyMatches}
        <p class="muted">
          {sorted.length === 0 ? `No installed apps match “${filter}”.` : 'No apps in this view.'}
        </p>
      {:else if $settings.installedGroup}
        {#if acyMatches}
          <div class="group-head">
            <span class="acy-badge">Acy</span>
            <span class="count soft">1</span>
          </div>
          <div class={$settings.installedView === 'grid' ? 'grid-rows' : 'list'}>
            {@render acyRow()}
          </div>
        {/if}
        {#each visibleGroups as g (g.key)}
          <button
            class="group-head group-toggle-head"
            onclick={() => toggleGroup(g.key)}
            aria-expanded={!collapsedGroups.has(g.key)}
          >
            <ChevronDown size={15} class="chev {collapsedGroups.has(g.key) ? 'collapsed' : ''}" />
            {#if g.badge}
              <SourceBadge source={g.badge} />
            {:else}
              <span class="group-label">{g.label}</span>
            {/if}
            <span class="count soft">{g.items.length}</span>
          </button>
          {#if !collapsedGroups.has(g.key)}
          <div class={$settings.installedView === 'grid' ? 'grid-rows' : 'list'}>
            {#each g.items as p (p.source + p.id)}
              <div
                class="row card"
                class:selected={selected.has(selKey(p))}
                oncontextmenu={(e) => rowMenu(e, p)}
                role="group"
              >
                <AppIcon name={p.name} size={36} source={p.source} id={p.id} homepage={p.homepage} />
                <div class="info">
                  <a class="name" href={`/app/${p.source}/${encodeURIComponent(p.id)}?from=installed`}>{p.name}</a>
                  <div class="ver mono">{p.id}{p.version ? ` · ${p.version}` : ''}</div>
                  {#if dupeNote(p)}<div class="dupe">also via {dupeNote(p)}</div>{/if}
                </div>
                <div class="row-action">
                  {#if selectMode}
                    <label class="acheck row-select">
                      <input
                        type="checkbox"
                        checked={selected.has(selKey(p))}
                        onchange={() => toggleSel(selKey(p))}
                        aria-label={`Select ${p.name}`}
                      />
                      <span class="box"><Check size={13} /></span>
                      <span>{selected.has(selKey(p)) ? 'Selected' : 'Select'}</span>
                    </label>
                  {:else}
                    <InstallButton
                      source={p.source}
                      id={p.id}
                      name={p.name}
                      kind="uninstall"
                      onDone={refreshLibrary}
                    />
                  {/if}
                </div>
              </div>
            {/each}
          </div>
          {/if}
        {/each}
      {:else}
        <div class={$settings.installedView === 'grid' ? 'grid-rows' : 'list'}>
          {#if acyMatches}{@render acyRow()}{/if}
          {#each visibleSorted as p (p.source + p.id)}
            <div
              class="row card"
              class:selected={selected.has(selKey(p))}
              oncontextmenu={(e) => rowMenu(e, p)}
              role="group"
            >
              <AppIcon name={p.name} size={36} source={p.source} id={p.id} homepage={p.homepage} />
              <div class="info">
                <a class="name" href={`/app/${p.source}/${encodeURIComponent(p.id)}?from=installed`}>{p.name}</a>
                <div class="ver mono">{p.id}{p.version ? ` · ${p.version}` : ''}</div>
                {#if dupeNote(p)}<div class="dupe">also via {dupeNote(p)}</div>{/if}
              </div>
              <SourceBadge source={p.source} />
              <div class="row-action">
                {#if selectMode}
                  <label class="acheck row-select">
                    <input
                      type="checkbox"
                      checked={selected.has(selKey(p))}
                      onchange={() => toggleSel(selKey(p))}
                      aria-label={`Select ${p.name}`}
                    />
                    <span class="box"><Check size={13} /></span>
                    <span>{selected.has(selKey(p)) ? 'Selected' : 'Select'}</span>
                  </label>
                {:else}
                  <InstallButton
                    source={p.source}
                    id={p.id}
                    name={p.name}
                    kind="uninstall"
                    onDone={refreshLibrary}
                  />
                {/if}
              </div>
            </div>
          {/each}
        </div>
      {/if}
    {/if}
  </section>
{/if}

<style>
  .head {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 22px;
  }
  .head h1 {
    flex: 1;
  }
  .checked {
    font-size: 0.8rem;
  }
  .block {
    margin-bottom: 30px;
  }
  .block-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 12px;
    flex-wrap: wrap;
  }
  .block-head h2 {
    font-size: 1.05rem;
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .count {
    font-family: var(--font-mono);
    font-size: 0.78rem;
    background: var(--accent-fill);
    color: var(--accent-contrast);
    border-radius: var(--radius-sm);
    padding: 0 8px;
    line-height: 1.55;
  }
  .count.soft {
    background: var(--surface-hover);
    color: var(--text-muted);
  }
  .all-actions {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }
  .batch-progress {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    margin: -2px 0 10px;
    color: var(--text-muted);
    font-size: 0.82rem;
  }
  .uptodate {
    color: var(--success);
    font-size: 0.9rem;
    margin: -8px 0 24px;
  }
  .icon-action {
    display: inline-flex;
    padding: 6px;
    border: 0;
    border-radius: var(--radius-sm);
    color: var(--text-muted);
    background: transparent;
  }
  .icon-action:hover {
    color: var(--text);
    background: var(--surface-hover);
  }
  .ignored-block {
    margin: -12px 0 28px;
  }
  .ignored-toggle {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    padding: 4px 0;
    border: 0;
    background: transparent;
    color: var(--text-muted);
    font-size: 0.86rem;
  }
  .ignored-toggle:hover {
    color: var(--text);
  }
  .ignored-list {
    margin-top: 9px;
  }
  .toolbar {
    display: flex;
    align-items: center;
    gap: 14px;
    margin-bottom: 12px;
    flex-wrap: wrap;
  }
  .filter {
    flex: 1;
    min-width: 200px;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    color: var(--text);
    padding: 8px 12px;
    font-size: 0.9rem;
    outline: none;
  }
  .filter:focus {
    border-color: var(--accent);
  }
  .sort,
  .group-toggle {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    font-size: 0.86rem;
    color: var(--text-muted);
    white-space: nowrap;
  }
  .sort select {
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    color: var(--text);
    padding: 6px 8px;
    font-size: 0.86rem;
  }
  .group-toggle {
    cursor: pointer;
  }
  .switch {
    position: relative;
    display: inline-block;
    width: 40px;
    height: 22px;
    flex-shrink: 0;
  }
  .switch input {
    opacity: 0;
    width: 0;
    height: 0;
  }
  .slider {
    position: absolute;
    inset: 0;
    cursor: pointer;
    background: var(--border-strong);
    border-radius: var(--radius-pill);
    transition: background 0.15s;
  }
  .slider::before {
    content: '';
    position: absolute;
    height: 16px;
    width: 16px;
    left: 3px;
    top: 3px;
    background: #fff;
    border-radius: 50%;
    transition: transform 0.15s;
  }
  .switch input:checked + .slider {
    background: var(--accent-fill);
  }
  .switch input:checked + .slider::before {
    transform: translateX(18px);
  }
  .group-head {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 16px 0 8px;
  }
  .group-head:first-child {
    margin-top: 0;
  }
  .group-toggle-head {
    width: 100%;
    background: none;
    border: none;
    cursor: pointer;
    padding: 4px 0;
    color: var(--text-muted);
    font: inherit;
  }
  .group-toggle-head:hover {
    color: var(--text);
  }
  .group-toggle-head .group-label {
    font-size: 0.85rem;
    font-weight: 600;
  }
  .group-toggle-head :global(.chev) {
    transition: transform 0.15s;
  }
  .group-toggle-head :global(.chev.collapsed) {
    transform: rotate(-90deg);
  }
  .dupe {
    font-size: 0.72rem;
    color: var(--accent);
    margin-top: 2px;
  }
  .warn-banner {
    background: color-mix(in srgb, var(--warning, #d97706) 12%, transparent);
    border: 1px solid color-mix(in srgb, var(--warning, #d97706) 40%, transparent);
    color: var(--text);
    border-radius: var(--radius);
    padding: 8px 12px;
    margin-bottom: 14px;
    font-size: 0.85rem;
  }
  .list {
    display: flex;
    flex-direction: column;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    overflow: hidden;
  }
  /* In list view the rows are dividers inside one bordered container (like the
     package-manager list), not separate cards. */
  .list .row {
    border: none;
    border-radius: 0;
    box-shadow: none;
    border-top: 1px solid var(--border);
  }
  .list .row:first-child {
    border-top: none;
  }
  .list .row.selected {
    box-shadow: inset 2px 0 0 var(--accent);
  }
  .grid-rows {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
    gap: 8px;
  }
  .tb-spacer {
    flex: 1;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 14px;
  }
  .acy-icon {
    width: 36px;
    height: 36px;
    flex-shrink: 0;
    object-fit: contain;
  }
  .acy-badge {
    display: inline-flex;
    align-items: center;
    width: fit-content;
    padding: 1px 8px;
    border: 1px solid var(--accent);
    border-radius: var(--radius-sm);
    color: var(--accent);
    font-family: var(--font-mono);
    font-size: 0.7rem;
    line-height: 1.45;
  }
  .current-app {
    color: var(--text-muted);
    font-size: 0.78rem;
    white-space: nowrap;
  }
  .row-action {
    width: 100px;
    height: 39px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: flex-end;
  }
  .row-select {
    width: 100%;
    justify-content: flex-end;
    gap: 7px;
    color: var(--text-muted);
    font-size: 0.8rem;
    font-weight: 500;
  }
  .info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .name {
    font-weight: 600;
    color: inherit;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .ver {
    font-size: 0.74rem;
    color: var(--text-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .error {
    color: var(--danger);
    font-family: var(--font-mono);
    font-size: 0.85rem;
    white-space: pre-wrap;
  }
  .sel-bar {
    position: sticky;
    top: 64px;
    z-index: 5;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 14px;
    margin-bottom: 12px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }
  .sel-count {
    font-size: 0.9rem;
    font-weight: 600;
  }
  .sel-spacer {
    flex: 1;
  }
  .row.selected {
    border-color: var(--accent);
  }
  .sk-icon {
    width: 36px;
    height: 36px;
    border-radius: var(--radius-sm);
    flex-shrink: 0;
  }
  .sk-lines {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 7px;
  }
  .sk-line {
    height: 11px;
  }
  .sk-line.lg {
    width: 45%;
  }
  .sk-line.sm {
    width: 30%;
  }
</style>

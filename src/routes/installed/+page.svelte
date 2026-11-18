<script lang="ts">
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { Search, X, Ellipsis, RefreshCw, EyeOff, PackageOpen } from '@lucide/svelte';
  import AppIcon from '$lib/components/AppIcon.svelte';
  import SourceBadge from '$lib/components/SourceBadge.svelte';
  import InstallButton from '$lib/components/InstallButton.svelte';
  import type { Package } from '$lib/types';
  import { settings, hideApp, unhideApp } from '$lib/stores/settings';
  import {
    installed,
    installedLoading,
    installedReady,
    installedError,
    updates,
    loadInstalled,
    refreshLibrary,
    isAcyPackage
  } from '$lib/stores/library';
  import { BUCKET_BY_KEY, bucketKey } from '$lib/installedGroups';
  import { runOp, summarizeBatch, installCommand } from '$lib/install';
  import { confirmAction } from '$lib/stores/confirm';
  import { copyText } from '$lib/clipboard';
  import { openContextMenu, openContextMenuAt, type CtxItem } from '$lib/stores/contextMenu';

  $effect(() => {
    loadInstalled();
  });

  type SortKey = 'name' | 'name-desc' | 'updates' | 'source';
  const SORTS: { value: SortKey; label: string }[] = [
    { value: 'name', label: 'Name (A to Z)' },
    { value: 'name-desc', label: 'Name (Z to A)' },
    { value: 'updates', label: 'Updates first' },
    { value: 'source', label: 'Source' }
  ];

  let group = $derived($page.url.searchParams.get('group') ?? 'all');
  let groupLabel = $derived(
    group === 'all' ? 'Installed' : (BUCKET_BY_KEY.get(group as never)?.label ?? 'Installed')
  );

  let filter = $state('');
  let showHidden = $state(false);
  let sortKey = $state<SortKey>('name');

  const keyOf = (p: Package) => `${p.source}:${p.id.toLowerCase()}`;
  // ARP\… and MSIX\… ids are Windows' internal install records, not something to show.
  const isRecordId = (id: string) => /^(ARP|MSIX)\\/i.test(id);
  let hiddenSet = $derived(new Set($settings.hiddenApps));
  let updateBy = $derived(new Map($updates.map((u) => [keyOf(u), u])));

  let inGroup = $derived($installed.filter((p) => group === 'all' || bucketKey(p) === group));
  let hiddenCount = $derived(inGroup.filter((p) => hiddenSet.has(keyOf(p))).length);
  let fq = $derived(filter.trim().toLowerCase());
  let rows = $derived.by(() => {
    const list = inGroup.filter(
      (p) =>
        (showHidden || !hiddenSet.has(keyOf(p))) &&
        (!fq ||
          p.name.toLowerCase().includes(fq) ||
          p.id.toLowerCase().includes(fq) ||
          (p.publisher ?? '').toLowerCase().includes(fq))
    );
    const byName = (a: Package, b: Package) =>
      a.name.localeCompare(b.name, undefined, { numeric: true, sensitivity: 'base' });
    return list.sort((a, b) => {
      switch (sortKey) {
        case 'name-desc':
          return byName(b, a);
        case 'updates':
          return Number(updateBy.has(keyOf(b))) - Number(updateBy.has(keyOf(a))) || byName(a, b);
        case 'source':
          return a.source.localeCompare(b.source) || byName(a, b);
        default:
          return byName(a, b);
      }
    });
  });
  let updateCount = $derived(rows.filter((p) => updateBy.has(keyOf(p))).length);

  // --- Selection & bulk uninstall ------------------------------------------
  let selected = $state<Set<string>>(new Set());
  let removing = $state<{ current: number; total: number } | null>(null);
  let selectable = $derived(rows.filter((p) => !isAcyPackage(p)));
  let selectedRows = $derived(selectable.filter((p) => selected.has(keyOf(p))));
  let allSelected = $derived(selectable.length > 0 && selectedRows.length === selectable.length);
  // Drop selections that the filter or group no longer shows.
  $effect(() => {
    const visible = new Set(selectable.map(keyOf));
    if ([...selected].some((k) => !visible.has(k))) {
      selected = new Set([...selected].filter((k) => visible.has(k)));
    }
  });

  function toggle(p: Package) {
    const next = new Set(selected);
    if (next.has(keyOf(p))) next.delete(keyOf(p));
    else next.add(keyOf(p));
    selected = next;
  }
  function toggleAll() {
    selected = allSelected ? new Set() : new Set(selectable.map(keyOf));
  }

  async function uninstallSelected() {
    const list = selectedRows;
    const ok = await confirmAction({
      title: `Uninstall ${list.length} ${list.length === 1 ? 'app' : 'apps'}?`,
      message: list.map((p) => p.name).join(', '),
      confirmLabel: `Uninstall ${list.length}`,
      danger: true
    });
    if (!ok) return;
    let done = 0;
    for (const [i, p] of list.entries()) {
      removing = { current: i + 1, total: list.length };
      if (await runOp('uninstall', p.source, p.id, p.name)) done++;
    }
    removing = null;
    selected = new Set();
    refreshLibrary();
    if (list.length > 1) summarizeBatch(list.length, done, 'removed');
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
    const ok = await confirmAction({ title: `Uninstall ${p.name}?`, confirmLabel: 'Uninstall', danger: true });
    if (!ok) return;
    await runOp('uninstall', p.source, p.id, p.name);
    refreshLibrary();
  }

  let back = $derived($page.url.pathname + $page.url.search);
  const detailHref = (p: Package) =>
    `/app/${p.source}/${encodeURIComponent(p.id)}?back=${encodeURIComponent(back)}`;

  function menuFor(p: Package): CtxItem[] {
    const items: CtxItem[] = [{ label: 'Open details', onSelect: () => goto(detailHref(p)) }];
    const cmd = installCommand(p.source, p.id);
    if (cmd) items.push({ label: 'Copy command', onSelect: () => copyText(cmd) });
    items.push({ label: 'Copy id', onSelect: () => copyText(p.id) });
    if (p.homepage) items.push({ label: 'Open homepage', onSelect: () => openUrl(p.homepage!) });
    items.push(
      hiddenSet.has(keyOf(p))
        ? { label: 'Unhide from list', onSelect: () => unhideApp(keyOf(p)) }
        : { label: 'Hide from list', onSelect: () => hideApp(keyOf(p)) }
    );
    items.push({ label: 'Uninstall…', danger: true, onSelect: () => uninstallOne(p) });
    return items;
  }

  function meta(p: Package): string {
    const parts: string[] = [];
    if (p.version && p.version.toLowerCase() !== 'unknown') parts.push(p.version);
    // The fast winget list reports the source ("winget") as publisher; the
    // badge already says that, so show the package id instead.
    const publisher = p.publisher && p.publisher.toLowerCase() !== p.source ? p.publisher : null;
    if (publisher) parts.push(publisher);
    else if (!isRecordId(p.id)) parts.push(p.id);
    return parts.join(' · ');
  }
</script>

<div class="view">
  <header class="view-head">
    <div>
      <h1>{groupLabel}</h1>
      <p class="view-sub">
        {rows.length} {rows.length === 1 ? 'app' : 'apps'}{updateCount > 0
          ? ` · ${updateCount} with updates`
          : ''}{$installedLoading ? ' · refreshing…' : ''}
      </p>
    </div>
    <button class="btn" onclick={() => refreshLibrary()} disabled={$installedLoading}>
      <RefreshCw size={15} class={$installedLoading ? 'spinning' : ''} /> Refresh
    </button>
  </header>

  <div class="toolbar">
    <div class="filter">
      <Search size={15} aria-hidden="true" />
      <input
        placeholder="Filter by name, publisher or id"
        aria-label="Filter installed apps"
        bind:value={filter}
        onkeydown={(e) => {
          if (e.key === 'Escape' && filter) {
            e.preventDefault();
            filter = '';
          }
        }}
      />
      {#if filter}
        <button class="icon-button tiny" onclick={() => (filter = '')} aria-label="Clear filter"><X size={14} /></button>
      {/if}
    </div>
    <label class="sort">
      <span class="muted">Sort by</span>
      <select class="text-input" bind:value={sortKey}>
        {#each SORTS as s (s.value)}<option value={s.value}>{s.label}</option>{/each}
      </select>
    </label>
    {#if hiddenCount > 0}
      <button class="btn btn-ghost" onclick={() => (showHidden = !showHidden)} aria-pressed={showHidden}>
        <EyeOff size={15} />
        {showHidden ? 'Hide hidden apps' : `Show ${hiddenCount} hidden`}
      </button>
    {/if}
  </div>

  {#if selectable.length > 0 && rows.length > 0}
    <div class="selbar" class:active={selectedRows.length > 0} role="region" aria-label="Selection">
      <label class="check-all">
        <input
          class="check-box"
          type="checkbox"
          checked={allSelected}
          indeterminate={selectedRows.length > 0 && !allSelected}
          onchange={toggleAll}
        />
        <span>
          {#if removing}
            Uninstalling {removing.current} of {removing.total}…
          {:else if selectedRows.length > 0}
            {selectedRows.length} selected
          {:else}
            Select all
          {/if}
        </span>
      </label>
      {#if selectedRows.length > 0}
        <div class="spacer"></div>
        <button class="btn btn-ghost" onclick={() => (selected = new Set())} disabled={!!removing}>Clear</button>
        <button class="btn danger-btn" onclick={uninstallSelected} disabled={!!removing}>
          Uninstall {selectedRows.length}
        </button>
      {/if}
    </div>
  {/if}

  <div class="view-body">
    <div class="page">
      {#if $installedError && $installed.length === 0}
        <div class="empty">
          <h2>Couldn't read installed apps</h2>
          <p class="muted">{$installedError}</p>
          <button class="btn" onclick={() => refreshLibrary()}>Try again</button>
        </div>
      {:else if !$installedReady && $installed.length === 0}
        <ul class="rows" aria-busy="true" aria-label="Loading installed apps">
          {#each Array(8) as _, i (i)}
            <li class="row sk"><div class="skeleton sk-icon"></div><div class="skeleton sk-line"></div></li>
          {/each}
        </ul>
      {:else if rows.length === 0}
        <div class="empty">
          <PackageOpen size={28} aria-hidden="true" />
          <h2>{filter ? `No installed apps match “${filter}”` : 'Nothing here'}</h2>
          <p class="muted">{filter ? 'Try a shorter name, or clear the filter.' : 'No installed apps in this group.'}</p>
        </div>
      {:else}
        <ul class="rows" class:selecting={selectedRows.length > 0}>
          {#each rows as p (keyOf(p))}
            {@const upd = updateBy.get(keyOf(p))}
            {@const hidden = hiddenSet.has(keyOf(p))}
            <li
              class="row"
              class:selected={selected.has(keyOf(p))}
              class:hidden-row={hidden}
            >
              <!-- The name link stretches over the card (see .name::after), so the
                   whole row opens the app; its controls sit above that layer. -->
              <div class="row-inner" role="group" aria-label={p.name} oncontextmenu={(e) => openContextMenu(e, menuFor(p))}>
              <span class="check">
                {#if !isAcyPackage(p)}
                  <input class="check-box" type="checkbox" checked={selected.has(keyOf(p))} onchange={() => toggle(p)} aria-label={`Select ${p.name}`} />
                {/if}
              </span>
              <AppIcon name={p.name} size={32} source={p.source} id={p.id} homepage={p.homepage} />
              <span class="text">
                <a class="name" href={detailHref(p)}>{p.name}</a>
                <span class="meta">
                  {#if hidden}<span class="pill">Hidden</span>{/if}
                  {meta(p)}
                </span>
              </span>
              {#if upd}
                <span class="upd">
                  <span class="upd-text">Update to <span class="mono">{upd.availableVersion ?? '?'}</span></span>
                  <InstallButton source={p.source} id={p.id} name={p.name} kind="update" quiet onDone={() => refreshLibrary()} />
                </span>
              {/if}
              <SourceBadge source={p.source} id={p.id} />
              <button
                class="icon-button"
                aria-label={`More actions for ${p.name}`}
                aria-haspopup="menu"
                onclick={(e) => {
                  e.stopPropagation();
                  openContextMenuAt(e.currentTarget, menuFor(p));
                }}
              >
                <Ellipsis size={16} />
              </button>
              </div>
            </li>
          {/each}
        </ul>
      {/if}
    </div>
  </div>
</div>

<style>
  .page {
    max-width: 960px;
    padding: 0 24px 32px;
  }
  .view-head :global(.spinning) {
    animation: spin 1s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  .toolbar {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 10px;
    max-width: 960px;
    padding: 0 24px 12px;
  }
  .filter {
    display: flex;
    align-items: center;
    gap: 8px;
    flex: 1;
    min-width: 220px;
    max-width: 420px;
    height: 36px;
    padding: 0 6px 0 10px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface);
    color: var(--text-muted);
  }
  .filter:focus-within {
    border-color: var(--accent);
    box-shadow: 0 0 0 1px var(--accent);
  }
  .filter input {
    flex: 1;
    min-width: 0;
    border: none;
    outline: none;
    background: transparent;
    color: var(--text);
    font: inherit;
    font-size: var(--fs-base);
  }
  .tiny {
    width: 24px;
    height: 24px;
  }
  .sort {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: var(--fs-md);
  }
  .sort select {
    width: auto;
  }
  .selbar {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 10px;
    max-width: 960px;
    min-height: 44px;
    padding: 0 24px 6px;
  }
  .selbar.active {
    margin: 0 24px 8px;
    max-width: 912px;
    padding: 4px 8px 4px 16px;
    border-radius: var(--radius);
    background: color-mix(in srgb, var(--accent) 12%, transparent);
  }
  .check-all {
    display: flex;
    align-items: center;
    gap: 14px;
    padding-left: 17px;
    font-size: var(--fs-md);
    font-weight: 500;
    cursor: pointer;
  }
  .selbar.active .check-all {
    padding-left: 0;
  }
  .spacer {
    flex: 1;
  }
  .danger-btn {
    background: var(--danger-fill);
    border-color: var(--danger-fill);
    color: #fff;
  }
  .danger-btn:hover:not(:disabled) {
    background: var(--danger-fill-hover);
    border-color: var(--danger-fill-hover);
  }

  .rows {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .row {
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface-2);
    transition: background 0.12s;
  }
  .row-inner {
    position: relative;
    display: flex;
    align-items: center;
    gap: 14px;
    min-height: 60px;
    padding: 8px 10px 8px 16px;
  }
  .name::after {
    content: '';
    position: absolute;
    inset: 0;
    border-radius: var(--radius);
  }
  .name:hover {
    text-decoration: none;
  }
  /* Controls stay clickable above the stretched link. */
  .check,
  .upd,
  .row-inner > .icon-button {
    position: relative;
    z-index: 1;
  }
  .row:hover {
    background: var(--surface-hover);
  }
  .row.selected {
    background: color-mix(in srgb, var(--accent) 12%, transparent);
    border-color: color-mix(in srgb, var(--accent) 40%, var(--border));
  }
  .check {
    flex-shrink: 0;
    display: inline-flex;
    width: 18px;
  }
  /* Row checkboxes appear on hover/keyboard focus, or for every row once a
     selection has started; "Select all" above keeps selection discoverable. */
  .rows:not(.selecting) .row:not(:hover):not(:focus-within) .check input {
    opacity: 0;
  }
  .check input {
    transition: opacity 0.12s;
  }
  .text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .name {
    color: var(--text);
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .meta {
    font-size: var(--fs-sm);
    color: var(--text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .hidden-row .name,
  .hidden-row :global(.icon) {
    opacity: 0.6;
  }
  .pill {
    margin-right: 6px;
    padding: 0 7px;
    border-radius: var(--radius-pill);
    background: color-mix(in srgb, var(--text) 9%, transparent);
    font-size: var(--fs-2xs);
    font-weight: 600;
  }
  .upd {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .upd-text {
    font-size: var(--fs-sm);
    color: var(--accent);
  }
  .upd :global(.btn) {
    padding: 4px 12px;
    font-size: var(--fs-sm);
  }
  .row :global(.badge) {
    flex-shrink: 0;
  }

  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    margin-top: 48px;
    text-align: center;
    color: var(--text-muted);
  }
  .empty h2 {
    margin: 6px 0 0;
    font-size: var(--fs-lg);
    color: var(--text);
  }
  .empty p {
    margin: 0 0 12px;
    font-size: var(--fs-md);
    overflow-wrap: anywhere;
  }
  .sk {
    display: flex;
    align-items: center;
    gap: 14px;
    min-height: 60px;
    padding: 8px 16px;
  }
  .sk-icon {
    width: 32px;
    height: 32px;
    margin-left: 30px;
  }
  .sk-line {
    width: 40%;
    height: 11px;
  }
</style>

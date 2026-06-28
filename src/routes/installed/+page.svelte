<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { goto } from '$app/navigation';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import * as api from '$lib/api';
  import type { Source, Package } from '$lib/types';
  import { enqueue } from '$lib/stores/ops';
  import { runOp, summarizeBatch } from '$lib/install';
  import { copyText } from '$lib/clipboard';
  import { openContextMenu } from '$lib/stores/contextMenu';
  import {
    settings,
    setInstalledSort,
    setInstalledGroup,
    setInstalledView
  } from '$lib/stores/settings';
  import {
    installed,
    updates,
    installedLoading,
    updatesLoading,
    installedError,
    updatesError,
    installedReady,
    lastChecked,
    loadInstalled,
    loadUpdates,
    refreshLibrary
  } from '$lib/stores/library';
  import { Check } from '@lucide/svelte';
  import InstallButton from '$lib/components/InstallButton.svelte';
  import AppIcon from '$lib/components/AppIcon.svelte';
  import SourceBadge from '$lib/components/SourceBadge.svelte';
  import ViewToggle from '$lib/components/ViewToggle.svelte';

  let updatingAll = $state<Source | null>(null);
  let updatingEverything = $state(false);

  // Filter is transient; sort + grouping persist via settings.
  let filter = $state('');

  let loading = $derived($installedLoading || $updatesLoading);
  let error = $derived($installedError ?? $updatesError);
  let updateSources = $derived([...new Set($updates.map((u) => u.source))]);

  const sourceOrder: Source[] = ['winget', 'scoop', 'choco', 'msstore'];
  const sourceNames: Record<Source, string> = {
    winget: 'winget',
    scoop: 'Scoop',
    choco: 'Chocolatey',
    msstore: 'Microsoft Store',
    local: 'Local file'
  };

  let q = $derived(filter.trim().toLowerCase());
  let filtered = $derived(
    $installed.filter(
      (p) => !q || p.name.toLowerCase().includes(q) || p.id.toLowerCase().includes(q)
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
  let grouped = $derived(
    sourceOrder
      .map((s) => ({ source: s, items: sorted.filter((p) => p.source === s) }))
      .filter((g) => g.items.length > 0)
  );

  // A coarse clock so "checked … ago" stays roughly current without re-rendering
  // constantly.
  let now = $state(Date.now());

  // Cache-aware: instant on repeat visits, fetches only the first time.
  onMount(() => {
    loadInstalled();
    loadUpdates();
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
    updatingAll = source;
    const names = $updates.filter((u) => u.source === source).map((u) => u.name);
    await enqueue(
      `Update all (${source})`,
      (opId) => api.upgradeAll(source, opId),
      undefined,
      names.join(', '),
      { action: 'update-all', name: sourceNames[source], source }
    );
    updatingAll = null;
    refreshLibrary();
  }

  // Update every manager that has pending updates, one after another.
  async function updateEverything() {
    updatingEverything = true;
    for (const s of updateSources) {
      await updateAll(s);
    }
    updatingEverything = false;
  }

  // ---- Multi-select uninstall ----
  function selKey(p: Package) {
    return `${p.source}:${p.id}`;
  }
  let selected = $state<Set<string>>(new Set());
  let selectedCount = $derived(selected.size);
  let allVisibleSelected = $derived(
    sorted.length > 0 && sorted.every((p) => selected.has(selKey(p)))
  );

  function toggleSel(k: string) {
    const next = new Set(selected);
    if (next.has(k)) next.delete(k);
    else next.add(k);
    selected = next;
  }
  function toggleAll() {
    const next = new Set(selected);
    if (allVisibleSelected) {
      for (const p of sorted) next.delete(selKey(p));
    } else {
      for (const p of sorted) next.add(selKey(p));
    }
    selected = next;
  }
  function clearSel() {
    selected = new Set();
  }

  // Batch uninstall: one confirm for the whole selection, then run sequentially.
  let removing = $state(false);
  let confirmingBatch = $state(false);
  let batchTimer: ReturnType<typeof setTimeout> | undefined;
  onDestroy(() => clearTimeout(batchTimer));

  function batchClick() {
    if (!confirmingBatch) {
      confirmingBatch = true;
      clearTimeout(batchTimer);
      batchTimer = setTimeout(() => (confirmingBatch = false), 3500);
      return;
    }
    clearTimeout(batchTimer);
    confirmingBatch = false;
    uninstallSelected();
  }

  async function uninstallSelected() {
    removing = true;
    const targets = $installed.filter((p) => selected.has(selKey(p)));
    let ok = 0;
    for (const p of targets) {
      if (await runOp('uninstall', p.source, p.id, p.name)) ok++;
    }
    removing = false;
    clearSel();
    refreshLibrary();
    if (targets.length > 1) summarizeBatch(targets.length, ok, 'removed');
  }

  async function uninstallOne(p: Package) {
    await runOp('uninstall', p.source, p.id, p.name);
    refreshLibrary();
  }
  function rowMenu(e: MouseEvent, p: Package) {
    openContextMenu(e, [
      { label: 'Uninstall', danger: true, onSelect: () => uninstallOne(p) },
      {
        label: 'Open details',
        onSelect: () => goto(`/app/${p.source}/${encodeURIComponent(p.id)}`)
      },
      { label: 'Copy id', onSelect: () => copyText(p.id) },
      ...(p.homepage ? [{ label: 'Open homepage', onSelect: () => openUrl(p.homepage!) }] : [])
    ]);
  }
</script>

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
{:else if error}
  <p class="error">{error}</p>
{:else}
  {#if $updates.length > 0}
    <section class="block">
      <div class="block-head">
        <h2>Updates <span class="count">{$updates.length}</span></h2>
        <div class="all-actions">
          {#if updateSources.length > 1}
            <button
              class="btn btn-accent"
              onclick={updateEverything}
              disabled={updatingEverything || updatingAll !== null}
            >
              {updatingEverything ? 'Updating…' : `Update everything · ${$updates.length}`}
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
      <div class="list">
        {#each $updates as p (p.source + p.id)}
          <div class="row card">
            <AppIcon name={p.name} size={36} source={p.source} id={p.id} homepage={p.homepage} />
            <div class="info">
              <a class="name" href={`/app/${p.source}/${encodeURIComponent(p.id)}`}>{p.name}</a>
              <div class="ver mono">{p.version ?? '?'} → {p.availableVersion ?? '?'}</div>
            </div>
            <SourceBadge source={p.source} />
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

  {#if $updates.length === 0 && $installed.length > 0}
    <p class="uptodate">✓ Everything is up to date.</p>
  {/if}

  <section class="block">
    <div class="block-head">
      <h2>All installed <span class="count soft">{$installed.length}</span></h2>
    </div>
    {#if $installed.length === 0}
      <p class="muted">Nothing reported by the enabled managers.</p>
    {:else}
      {#if selectedCount > 0}
        <div class="sel-bar">
          <span class="sel-count">{selectedCount} selected</span>
          <div class="sel-spacer"></div>
          <button class="btn btn-ghost" onclick={clearSel} disabled={removing}>Clear</button>
          <button
            class="btn"
            class:confirm={confirmingBatch}
            onclick={batchClick}
            disabled={removing}
          >
            {removing
              ? 'Removing…'
              : confirmingBatch
                ? `Confirm remove ${selectedCount}?`
                : `Uninstall ${selectedCount}`}
          </button>
        </div>
      {/if}
      <div class="toolbar">
        <label class="selall acheck">
          <input type="checkbox" checked={allVisibleSelected} onchange={toggleAll} />
          <span class="box"><Check size={13} /></span>
          Select all
        </label>
        <input class="filter" placeholder="Filter installed…" bind:value={filter} />
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
        <label class="group-toggle">
          Group by manager
          <span class="switch">
            <input
              type="checkbox"
              checked={$settings.installedGroup}
              onchange={(e) => setInstalledGroup(e.currentTarget.checked)}
            />
            <span class="slider"></span>
          </span>
        </label>
        <div class="tb-spacer"></div>
        <ViewToggle value={$settings.installedView} onChange={setInstalledView} />
      </div>

      {#if sorted.length === 0}
        <p class="muted">No installed apps match “{filter}”.</p>
      {:else if $settings.installedGroup}
        {#each grouped as g (g.source)}
          <div class="group-head">
            <SourceBadge source={g.source} />
            <span class="count soft">{g.items.length}</span>
          </div>
          <div class={$settings.installedView === 'grid' ? 'grid-rows' : 'list'}>
            {#each g.items as p (p.source + p.id)}
              <div
                class="row card"
                class:selected={selected.has(selKey(p))}
                oncontextmenu={(e) => rowMenu(e, p)}
                role="group"
              >
                <label class="acheck sel">
                  <input
                    type="checkbox"
                    checked={selected.has(selKey(p))}
                    onchange={() => toggleSel(selKey(p))}
                  />
                  <span class="box"><Check size={13} /></span>
                </label>
                <AppIcon name={p.name} size={36} source={p.source} id={p.id} homepage={p.homepage} />
                <div class="info">
                  <a class="name" href={`/app/${p.source}/${encodeURIComponent(p.id)}`}>{p.name}</a>
                  <div class="ver mono">{p.id}{p.version ? ` · ${p.version}` : ''}</div>
                </div>
                <InstallButton
                  source={p.source}
                  id={p.id}
                  name={p.name}
                  kind="uninstall"
                  onDone={refreshLibrary}
                />
              </div>
            {/each}
          </div>
        {/each}
      {:else}
        <div class={$settings.installedView === 'grid' ? 'grid-rows' : 'list'}>
          {#each sorted as p (p.source + p.id)}
            <div
              class="row card"
              class:selected={selected.has(selKey(p))}
              oncontextmenu={(e) => rowMenu(e, p)}
              role="group"
            >
              <label class="acheck sel">
                <input
                  type="checkbox"
                  checked={selected.has(selKey(p))}
                  onchange={() => toggleSel(selKey(p))}
                />
                <span class="box"><Check size={13} /></span>
              </label>
              <AppIcon name={p.name} size={36} source={p.source} id={p.id} homepage={p.homepage} />
              <div class="info">
                <a class="name" href={`/app/${p.source}/${encodeURIComponent(p.id)}`}>{p.name}</a>
                <div class="ver mono">{p.id}{p.version ? ` · ${p.version}` : ''}</div>
              </div>
              <SourceBadge source={p.source} />
              <InstallButton
                source={p.source}
                id={p.id}
                name={p.name}
                kind="uninstall"
                onDone={refreshLibrary}
              />
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
    background: var(--accent);
    color: var(--accent-contrast);
    border-radius: var(--radius-pill);
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
  .uptodate {
    color: var(--success);
    font-size: 0.9rem;
    margin: -8px 0 24px;
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
    background: var(--accent);
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
  .list {
    display: flex;
    flex-direction: column;
    gap: 8px;
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
  .sel-bar .confirm {
    background: var(--danger);
    border-color: var(--danger);
    color: #fff;
  }
  .selall {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    font-size: 0.86rem;
    color: var(--text-muted);
    white-space: nowrap;
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

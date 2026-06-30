<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import type { Source, Package } from '$lib/types';
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
    refreshLibrary
  } from '$lib/stores/library';
  import { ignoreUpdate, restoreUpdate } from '$lib/stores/ignoredUpdates';
  import { Check, EyeOff, RotateCcw } from '@lucide/svelte';
  import InstallButton from '$lib/components/InstallButton.svelte';
  import ConfirmAction from '$lib/components/ConfirmAction.svelte';
  import AppIcon from '$lib/components/AppIcon.svelte';
  import SourceBadge from '$lib/components/SourceBadge.svelte';
  import ViewToggle from '$lib/components/ViewToggle.svelte';

  let updatingAll = $state<Source | null>(null);
  let updatingEverything = $state(false);

  let filter = $state('');

  let loading = $derived($installedLoading || $updatesLoading);
  let error = $derived($installedError ?? $updatesError);
  let updateSources = $derived([...new Set($actionableUpdates.map((u) => u.source))]);
  let batchProgress = $state<{ current: number; total: number; label: string } | null>(null);
  let showIgnored = $state(false);

  const sourceOrder: Source[] = ['winget', 'scoop', 'choco', 'msstore'];
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

  let now = $state(Date.now());

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

  async function updateEverything() {
    updatingEverything = true;
    for (const s of updateSources) {
      await updateAll(s);
    }
    updatingEverything = false;
  }

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
    if (!window.confirm(`Remove ${p.name}?`)) return;
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
      { label: 'Copy id', onSelect: () => copyText(p.id) },
      ...(p.homepage ? [{ label: 'Open homepage', onSelect: () => openUrl(p.homepage!) }] : [])
    ]);
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && selectMode) exitSelect();
  }
</script>

<svelte:window onkeydown={onKeydown} />

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
  {#if $actionableUpdates.length > 0}
    <section class="block">
      <div class="block-head">
        <h2>Updates <span class="count">{$actionableUpdates.length}</span></h2>
        <div class="all-actions">
          {#if updateSources.length > 1}
            <button
              class="btn btn-accent"
              onclick={updateEverything}
              disabled={updatingEverything || updatingAll !== null}
            >
              {updatingEverything ? 'Updating…' : `Update everything · ${$actionableUpdates.length}`}
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
        {#each $actionableUpdates as p (p.source + p.id)}
          <div class="row card">
            <AppIcon name={p.name} size={36} source={p.source} id={p.id} homepage={p.homepage} />
            <div class="info">
              <a class="name" href={`/app/${p.source}/${encodeURIComponent(p.id)}?from=installed`}>{p.name}</a>
              <div class="ver mono">{p.version ?? '?'} → {p.availableVersion ?? '?'}</div>
            </div>
            <SourceBadge source={p.source} />
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

  {#if $actionableUpdates.length === 0 && $installed.length > 0}
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
                <AppIcon name={p.name} size={36} source={p.source} id={p.id} homepage={p.homepage} />
                <div class="info">
                  <a class="name" href={`/app/${p.source}/${encodeURIComponent(p.id)}?from=installed`}>{p.name}</a>
                  <div class="ver mono">{p.id}{p.version ? ` · ${p.version}` : ''}</div>
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
              <AppIcon name={p.name} size={36} source={p.source} id={p.id} homepage={p.homepage} />
              <div class="info">
                <a class="name" href={`/app/${p.source}/${encodeURIComponent(p.id)}?from=installed`}>{p.name}</a>
                <div class="ver mono">{p.id}{p.version ? ` · ${p.version}` : ''}</div>
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

<script lang="ts">
  import { onMount } from 'svelte';
  import * as api from '$lib/api';
  import type { Source } from '$lib/types';
  import { enqueue } from '$lib/stores/ops';
  import {
    installed,
    updates,
    installedLoading,
    updatesLoading,
    installedError,
    updatesError,
    installedReady,
    loadInstalled,
    loadUpdates,
    refreshLibrary
  } from '$lib/stores/library';
  import InstallButton from '$lib/components/InstallButton.svelte';
  import AppIcon from '$lib/components/AppIcon.svelte';
  import SourceBadge from '$lib/components/SourceBadge.svelte';

  let updatingAll = $state<Source | null>(null);
  let updatingEverything = $state(false);

  let filter = $state('');
  let sortBy = $state<'name' | 'source'>('name');
  let groupByManager = $state(false);

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
      if (sortBy === 'source' && a.source !== b.source) {
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

  onMount(() => {
    loadInstalled();
    loadUpdates();
  });

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

  async function updateEverything() {
    updatingEverything = true;
    for (const s of updateSources) {
      await updateAll(s);
    }
    updatingEverything = false;
  }
</script>

<div class="head">
  <h1>Installed</h1>
  <button class="btn btn-ghost" onclick={() => refreshLibrary()} disabled={loading}>
    {loading ? 'Refreshing…' : 'Refresh'}
  </button>
</div>

{#if loading && !$installedReady}
  <p class="muted">Loading installed apps…</p>
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
      <div class="toolbar">
        <input class="filter" placeholder="Filter installed…" bind:value={filter} />
        <label class="sort">
          Sort
          <select bind:value={sortBy}>
            <option value="name">Name</option>
            <option value="source">Manager</option>
          </select>
        </label>
        <label class="group-toggle">
          Group by manager
          <span class="switch">
            <input type="checkbox" bind:checked={groupByManager} />
            <span class="slider"></span>
          </span>
        </label>
      </div>

      {#if sorted.length === 0}
        <p class="muted">No installed apps match “{filter}”.</p>
      {:else if groupByManager}
        {#each grouped as g (g.source)}
          <div class="group-head">
            <SourceBadge source={g.source} />
            <span class="count soft">{g.items.length}</span>
          </div>
          <div class="list">
            {#each g.items as p (p.source + p.id)}
              <div class="row card">
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
        <div class="list">
          {#each sorted as p (p.source + p.id)}
            <div class="row card">
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
    justify-content: space-between;
    margin-bottom: 22px;
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
</style>

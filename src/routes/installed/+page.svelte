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

  let loading = $derived($installedLoading || $updatesLoading);
  let error = $derived($installedError ?? $updatesError);
  let updateSources = $derived([...new Set($updates.map((u) => u.source))]);

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
      names.join(', ')
    );
    updatingAll = null;
    refreshLibrary();
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
          {#each updateSources as s (s)}
            <button class="btn" onclick={() => updateAll(s)} disabled={updatingAll === s}>
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

  <section class="block">
    <div class="block-head">
      <h2>All installed <span class="count soft">{$installed.length}</span></h2>
    </div>
    {#if $installed.length === 0}
      <p class="muted">Nothing reported by the enabled managers.</p>
    {:else}
      <div class="list">
        {#each $installed as p (p.source + p.id)}
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

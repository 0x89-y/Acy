<script lang="ts">
  import { onMount } from 'svelte';
  import { getVersion } from '@tauri-apps/api/app';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import type { Source, Package } from '$lib/types';
  import { runOp, summarizeBatch } from '$lib/install';
  import {
    installedLoading,
    updatesLoading,
    actionableUpdates,
    ignoredUpdates,
    lastChecked,
    refreshLibrary
  } from '$lib/stores/library';
  import { ignoreUpdate, restoreUpdate } from '$lib/stores/ignoredUpdates';
  import { updaterPhase, updaterVersion, installUpdate } from '$lib/stores/updater';
  import { curated, loadCurated } from '$lib/stores/curated';
  import { EyeOff, RotateCcw, FileText } from '@lucide/svelte';
  import InstallButton from '$lib/components/InstallButton.svelte';
  import AppIcon from '$lib/components/AppIcon.svelte';
  import SourceBadge from '$lib/components/SourceBadge.svelte';

  let updatingAll = $state<Source | null>(null);
  let updatingEverything = $state(false);
  let batchProgress = $state<{ current: number; total: number; label: string } | null>(null);
  let showIgnored = $state(false);
  let acyVersion = $state('');
  let loading = $derived($installedLoading || $updatesLoading);
  let acyUpdateAvailable = $derived($updaterPhase === 'available' && !!$updaterVersion);
  let totalUpdateCount = $derived($actionableUpdates.length + (acyUpdateAvailable ? 1 : 0));
  let updateSources = $derived([...new Set($actionableUpdates.map((u) => u.source))]);

  // Release-notes links from the curated catalog, keyed by every manager id.
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

  let now = $state(Date.now());
  onMount(() => {
    loadCurated();
    getVersion().then((v) => (acyVersion = v)).catch(() => (acyVersion = ''));
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
    for (const s of updateSources) await updateAll(s);
    if (acyUpdateAvailable) await installUpdate();
    updatingEverything = false;
  }
</script>

<div class="pane-head">
  {#if totalUpdateCount > 1}
    <button
      class="btn btn-accent"
      onclick={updateEverything}
      disabled={updatingEverything || updatingAll !== null || $updaterPhase === 'downloading'}
    >
      {updatingEverything ? 'Updating…' : `Update everything · ${totalUpdateCount}`}
    </button>
  {/if}
  <div class="spacer"></div>
  {#if $lastChecked}
    <span class="checked muted">Checked {ago($lastChecked, now)}</span>
  {/if}
  <button class="btn" onclick={() => refreshLibrary()} disabled={loading}>
    {loading ? 'Refreshing…' : 'Refresh'}
  </button>
</div>

<div class="pane-scroll">
  {#if batchProgress}
    <div class="batch-progress" role="status" aria-live="polite">
      <span>{batchProgress.label}</span>
      <span class="mono">{batchProgress.current} of {batchProgress.total}</span>
    </div>
  {/if}
  {#if totalUpdateCount > 0}
    <div class="list">
      {#if acyUpdateAvailable}
        <div class="row">
          <img class="acy-icon" src="/acy-icon.png" alt="" />
          <div class="info">
            <span class="name">Acy</span>
            <div class="ver mono">{acyVersion || '?'} → {$updaterVersion}</div>
          </div>
          <span class="acy-badge">Acy</span>
          <button class="btn btn-accent" onclick={installUpdate} disabled={$updaterPhase === 'downloading'}>
            {$updaterPhase === 'downloading' ? 'Installing…' : 'Update Acy'}
          </button>
        </div>
      {/if}
      {#each $actionableUpdates as p (p.source + p.id)}
        <div class="row">
          <AppIcon name={p.name} size={34} source={p.source} id={p.id} homepage={p.homepage} />
          <div class="info">
            <a class="name" href={`/app/${p.source}/${encodeURIComponent(p.id)}?back=${encodeURIComponent('/?view=library')}`}>{p.name}</a>
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
          <InstallButton source={p.source} id={p.id} name={p.name} kind="update" onDone={refreshLibrary} />
        </div>
      {/each}
    </div>
  {:else}
    <p class="uptodate">
      {$ignoredUpdates.length > 0 ? 'No active updates.' : '✓ Everything is up to date.'}
    </p>
  {/if}

  {#if $ignoredUpdates.length > 0}
    <div class="ignored-block">
      <button class="ignored-toggle" onclick={() => (showIgnored = !showIgnored)} aria-expanded={showIgnored}>
        Ignored updates <span class="count soft">{$ignoredUpdates.length}</span>
      </button>
      {#if showIgnored}
        <div class="list ignored-list">
          {#each $ignoredUpdates as p (p.source + p.id)}
            <div class="row">
              <AppIcon name={p.name} size={34} source={p.source} id={p.id} homepage={p.homepage} />
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
    </div>
  {/if}
</div>

<style>
  .pane-head {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px 10px;
    min-height: 34px;
    padding: 10px 14px;
    border-bottom: 1px solid var(--border);
  }
  .spacer {
    flex: 1;
  }
  .checked {
    font-size: 0.8rem;
  }
  .pane-scroll {
    flex: 1;
    min-height: 0;
    overflow: hidden auto;
  }
  .batch-progress {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    padding: 10px 14px;
    color: var(--text-muted);
    font-size: 0.82rem;
  }
  .uptodate {
    color: var(--success);
    font-size: 0.9rem;
    padding: 16px 14px;
  }
  .count {
    font-family: var(--font-mono);
    font-size: 0.78rem;
    background: var(--surface-hover);
    color: var(--text-muted);
    border-radius: var(--radius-sm);
    padding: 0 8px;
    line-height: 1.55;
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
    padding: 12px 14px 20px;
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
    border: 1px solid var(--border);
    border-radius: var(--radius);
    overflow: hidden;
  }
  .list {
    display: flex;
    flex-direction: column;
  }
  .list .row {
    border-top: 1px solid var(--border);
  }
  .list .row:first-child {
    border-top: none;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 14px;
  }
  .acy-icon {
    width: 34px;
    height: 34px;
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
</style>

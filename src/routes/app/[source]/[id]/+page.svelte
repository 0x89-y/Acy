<script lang="ts">
  import { page } from '$app/stores';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { ExternalLink, ArrowLeft } from '@lucide/svelte';
  import AppIcon from '$lib/components/AppIcon.svelte';
  import SourceBadge from '$lib/components/SourceBadge.svelte';
  import InstallButton from '$lib/components/InstallButton.svelte';
  import * as api from '$lib/api';
  import {
    installed as installedStore,
    updates as updatesStore,
    installedReady,
    loadInstalled,
    loadUpdates,
    refreshLibrary
  } from '$lib/stores/library';
  import { curated as curatedStore, loadCurated } from '$lib/stores/curated';
  import type { Package, Source } from '$lib/types';

  let source = $derived($page.params.source as Source);
  let id = $derived(decodeURIComponent($page.params.id ?? ''));
  let idLower = $derived(id.toLowerCase());

  // Hardcoded curated entry, if this app is one of ours.
  let curatedApp = $derived.by(() => {
    const c = $curatedStore;
    if (!c) return null;
    for (const cat of c.categories) {
      for (const app of cat.apps) {
        if (app.source === source && app.id.toLowerCase() === idLower) return app;
      }
    }
    return null;
  });
  let curatedKnown = $derived($curatedStore !== null);

  // Live details, fetched only for non-curated apps.
  let info = $state<Package | null>(null);
  let loadingInfo = $state(false);

  // Installed / update status from the shared cache (no fresh fetch).
  let installedPkg = $derived(
    $installedStore.find((p) => p.source === source && p.id.toLowerCase() === idLower) ?? null
  );
  let updatePkg = $derived(
    $updatesStore.find((p) => p.source === source && p.id.toLowerCase() === idLower) ?? null
  );

  let name = $derived(curatedApp?.name ?? info?.name ?? installedPkg?.name ?? id);
  let homepage = $derived(curatedApp?.homepage ?? info?.homepage ?? null);
  // Icon may come from a different place than the website (e.g. a fork installs
  // from one site but the recognizable logo lives on the original site).
  let iconSource = $derived(curatedApp?.icon ?? homepage);
  let description = $derived(curatedApp?.description ?? info?.description ?? null);
  let publisher = $derived(info?.publisher ?? null);
  let latestVersion = $derived(info?.version ?? null);
  let isInstalled = $derived(!!installedPkg);
  let updatable = $derived(!!updatePkg);

  $effect(() => {
    loadCurated();
    loadInstalled();
    loadUpdates();
  });

  // Only hit the slow info() lookup for non-curated apps.
  $effect(() => {
    const s = source;
    const i = id;
    if (!curatedKnown) return;
    if (curatedApp) {
      info = null;
      loadingInfo = false;
      return;
    }
    loadingInfo = true;
    info = null;
    api
      .appInfo(s, i)
      .then((d) => {
        if (source === s && id === i) info = d;
      })
      .catch(() => {})
      .finally(() => {
        loadingInfo = false;
      });
  });

  function onChanged() {
    refreshLibrary();
  }
</script>

<a class="back" href="/"><ArrowLeft size={16} /> Back</a>

{#if !curatedKnown}
  <p class="muted">Loading…</p>
{:else}
  <div class="header">
    <AppIcon {name} size={72} {source} {id} homepage={iconSource} />
    <div class="title">
      <h1>{name}</h1>
      <div class="meta">
        <SourceBadge {source} />
        <span class="mono id">{id}</span>
        {#if publisher}<span class="muted">· {publisher}</span>{/if}
      </div>
      <div class="actions">
        {#if $installedReady}
          {#if updatable}
            <InstallButton {source} {id} {name} kind="update" onDone={onChanged} />
          {/if}
          {#if isInstalled}
            <InstallButton {source} {id} {name} kind="uninstall" onDone={onChanged} />
          {:else}
            <InstallButton {source} {id} {name} kind="install" onDone={onChanged} />
          {/if}
        {:else}
          <span class="muted small">Checking status…</span>
        {/if}
        {#if homepage}
          <button class="btn btn-ghost" onclick={() => openUrl(homepage!)}>
            <ExternalLink size={15} /> Website
          </button>
        {/if}
      </div>
    </div>
  </div>

  <div class="facts card">
    <div class="fact">
      <span class="k">Status</span>
      <span class="v">
        {$installedReady ? (isInstalled ? 'Installed' : 'Not installed') : 'Checking…'}
      </span>
    </div>
    {#if installedPkg?.version}
      <div class="fact">
        <span class="k">Installed version</span>
        <span class="v mono">{installedPkg.version}</span>
      </div>
    {/if}
    {#if updatePkg?.availableVersion}
      <div class="fact">
        <span class="k">Update available</span>
        <span class="v mono">{updatePkg.availableVersion}</span>
      </div>
    {:else if latestVersion}
      <div class="fact">
        <span class="k">Latest version</span>
        <span class="v mono">{latestVersion}</span>
      </div>
    {/if}
    {#if homepage}
      <div class="fact">
        <span class="k">Website</span>
        <button class="link-btn mono" onclick={() => openUrl(homepage!)}>{homepage}</button>
      </div>
    {/if}
  </div>

  {#if description}
    <section class="desc-block">
      <h2>About</h2>
      <p>{description}</p>
    </section>
  {:else if loadingInfo}
    <p class="muted small">Loading details…</p>
  {:else}
    <p class="muted small">No description available.</p>
  {/if}
{/if}

<style>
  .back {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    color: var(--text-muted);
    text-decoration: none;
    font-size: 0.9rem;
    margin-bottom: 20px;
  }
  .back:hover {
    color: var(--text);
  }
  .header {
    display: flex;
    gap: 18px;
    margin-bottom: 24px;
  }
  .title {
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-width: 0;
  }
  .title h1 {
    font-size: 1.6rem;
  }
  .meta {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
    font-size: 0.85rem;
  }
  .meta .id {
    color: var(--text-muted);
    font-size: 0.78rem;
  }
  .actions {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    align-items: center;
    margin-top: 4px;
  }
  .facts {
    padding: 4px 16px;
    margin-bottom: 24px;
  }
  .fact {
    display: flex;
    gap: 16px;
    padding: 11px 0;
    border-bottom: 1px solid var(--border);
  }
  .fact:last-child {
    border-bottom: none;
  }
  .fact .k {
    width: 150px;
    flex-shrink: 0;
    color: var(--text-muted);
    font-size: 0.88rem;
  }
  .fact .v {
    min-width: 0;
    word-break: break-word;
  }
  .link-btn {
    background: none;
    border: none;
    padding: 0;
    color: var(--accent);
    cursor: pointer;
    font-size: 0.82rem;
    text-align: left;
    word-break: break-all;
  }
  .link-btn:hover {
    text-decoration: underline;
  }
  .desc-block h2 {
    font-size: 1.05rem;
    margin-bottom: 8px;
  }
  .desc-block p {
    color: var(--text);
    line-height: 1.6;
    max-width: 70ch;
  }
  .small {
    font-size: 0.85rem;
  }
</style>

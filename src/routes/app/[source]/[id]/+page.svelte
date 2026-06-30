<script lang="ts">
  import { page } from '$app/stores';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { ExternalLink, ArrowLeft, Copy, Check } from '@lucide/svelte';
  import AppIcon from '$lib/components/AppIcon.svelte';
  import SourceBadge from '$lib/components/SourceBadge.svelte';
  import InstallButton from '$lib/components/InstallButton.svelte';
  import InstallSplitButton from '$lib/components/InstallSplitButton.svelte';
  import { copyText } from '$lib/clipboard';
  import { installCommand } from '$lib/install';
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
  import { settings } from '$lib/stores/settings';
  import type { Package, Source, Variant } from '$lib/types';

  let source = $derived($page.params.source as Source);
  let id = $derived(decodeURIComponent($page.params.id ?? ''));
  let idLower = $derived(id.toLowerCase());
  let backHref = $derived.by(() => {
    const requested = $page.url.searchParams.get('back');
    if (requested?.startsWith('/') && !requested.startsWith('//')) return requested;
    return $page.url.searchParams.get('from') === 'installed' ? '/installed' : '/';
  });

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

  let variants = $derived.by<Variant[]>(() => {
    if (!curatedApp) return [{ source, id }];
    const seen = new Set<Source>();
    const out: Variant[] = [];
    for (const v of [{ source: curatedApp.source, id: curatedApp.id }, ...curatedApp.alternates]) {
      if ($settings.managers[v.source] === false || seen.has(v.source)) continue;
      seen.add(v.source);
      out.push(v);
    }
    return out.length > 0 ? out : [{ source, id }];
  });

  let info = $state<Package | null>(null);
  let loadingInfo = $state(false);

  function findPkg(list: Package[], v: Variant) {
    return list.find((p) => p.source === v.source && p.id.toLowerCase() === v.id.toLowerCase());
  }
  let installedVariant = $derived(variants.find((v) => findPkg($installedStore, v)) ?? null);
  let updateVariant = $derived(variants.find((v) => findPkg($updatesStore, v)) ?? null);
  let installedPkg = $derived(installedVariant ? findPkg($installedStore, installedVariant) ?? null : null);
  let updatePkg = $derived(updateVariant ? findPkg($updatesStore, updateVariant) ?? null : null);

  let name = $derived(curatedApp?.name ?? info?.name ?? installedPkg?.name ?? id);
  let homepage = $derived(curatedApp?.homepage ?? info?.homepage ?? null);
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

  let cmd = $derived(installCommand(source, id));
  let cmdCopied = $state(false);
  let idCopied = $state(false);

  async function copyCmd() {
    if (cmd && (await copyText(cmd))) {
      cmdCopied = true;
      setTimeout(() => (cmdCopied = false), 1200);
    }
  }
  async function copyId() {
    if (await copyText(id)) {
      idCopied = true;
      setTimeout(() => (idCopied = false), 1200);
    }
  }
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && history.back()} />

<a class="back" href={backHref}><ArrowLeft size={16} /> Back</a>

{#if !curatedKnown}
  <p class="muted">Loading…</p>
{:else}
  <div class="header">
    <AppIcon {name} size={72} {source} {id} homepage={iconSource} />
    <div class="title">
      <h1>{name}</h1>
      <div class="meta">
        {#each variants as v (v.source)}<SourceBadge source={v.source} />{/each}
        <span class="mono id">{id}</span>
        {#if publisher}<span class="muted">· {publisher}</span>{/if}
      </div>
      <div class="actions">
        {#if $installedReady}
          {#if updatable && updateVariant}
            <InstallButton
              source={updateVariant.source}
              id={updateVariant.id}
              {name}
              kind="update"
              onDone={onChanged}
            />
          {/if}
          {#if isInstalled && installedVariant}
            <InstallButton
              source={installedVariant.source}
              id={installedVariant.id}
              {name}
              kind="uninstall"
              onDone={onChanged}
            />
          {:else if !isInstalled}
            {#if variants.length > 1}
              <InstallSplitButton {variants} {name} preferred={$settings.preferredSource} onDone={onChanged} />
            {:else}
              <InstallButton {source} {id} {name} kind="install" onDone={onChanged} />
            {/if}
          {/if}
        {:else}
          <span class="muted small">Checking status…</span>
        {/if}
        {#if homepage}
          <button class="btn btn-ghost" onclick={() => openUrl(homepage!)}>
            <ExternalLink size={15} /> Website
          </button>
        {/if}
        {#if cmd}
          <button class="btn btn-ghost" onclick={copyCmd} title={cmd}>
            {#if cmdCopied}<Check size={15} /> Copied{:else}<Copy size={15} /> Copy command{/if}
          </button>
        {/if}
        <button class="btn btn-ghost" onclick={copyId} title="Copy package id">
          {#if idCopied}<Check size={15} /> Copied{:else}<Copy size={15} /> Copy id{/if}
        </button>
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

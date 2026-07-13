<script lang="ts">
  import { page } from '$app/stores';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { ExternalLink, ArrowLeft, Heart, Plus, Pencil, FileText } from '@lucide/svelte';
  import CuratedAppEditForm from '$lib/components/CuratedAppEditForm.svelte';
  import { filterByTag } from '$lib/stores/discover';
  import AppIcon from '$lib/components/AppIcon.svelte';
  import SourceBadge from '$lib/components/SourceBadge.svelte';
  import InstallButton from '$lib/components/InstallButton.svelte';
  import InstallSplitButton from '$lib/components/InstallSplitButton.svelte';
  import { copyText } from '$lib/clipboard';
  import { installCommand } from '$lib/install';
  import { openContextMenu, type CtxItem } from '$lib/stores/contextMenu';
  import * as api from '$lib/api';
  import {
    installed as installedStore,
    updates as updatesStore,
    installedReady,
    loadInstalled,
    loadUpdates,
    refreshLibrary
  } from '$lib/stores/library';
  import { curated as curatedStore, loadCurated, addToCurated } from '$lib/stores/curated';
  import { settings } from '$lib/stores/settings';
  import type { Package, Source, Variant } from '$lib/types';

  let source = $derived($page.params.source as Source);
  let id = $derived(decodeURIComponent($page.params.id ?? ''));
  let idLower = $derived(id.toLowerCase());
  let backHref = $derived.by(() => {
    const requested = $page.url.searchParams.get('back');
    if (requested?.startsWith('/') && !requested.startsWith('//')) return requested;
    return $page.url.searchParams.get('from') === 'installed' ? '/?view=library' : '/';
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
  let tags = $derived(curatedApp?.tags ?? []);
  let donate = $derived(curatedApp?.donate ?? null);
  let releaseNotes = $derived(curatedApp?.releaseNotes ?? null);
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

  let adding = $state(false);
  let editing = $state(false);
  async function addThis() {
    adding = true;
    const alternates = variants
      .filter((v) => !(v.source === source && v.id === id))
      .map((v) => ({ source: v.source, id: v.id }));
    await addToCurated({ source, id, name, description, homepage, alternates });
    adding = false;
  }

  let cmd = $derived(installCommand(source, id));

  function headerMenu(e: MouseEvent) {
    const items: CtxItem[] = [];
    if (cmd) items.push({ label: 'Copy command', onSelect: () => copyText(cmd!) });
    items.push({ label: 'Copy id', onSelect: () => copyText(id) });
    if (homepage) items.push({ label: 'Open homepage', onSelect: () => openUrl(homepage!) });
    openContextMenu(e, items);
  }
</script>

<svelte:window
  onkeydown={(e) => {
    if (e.key !== 'Escape') return;
    if (editing) editing = false;
    else history.back();
  }}
/>

<div class="browse-panel">
  <div class="side">
    <div class="rail-head">
      <a class="back-btn" href={backHref} title="Back" aria-label="Back"><ArrowLeft size={17} /></a>
      <div class="spacer"></div>
      {#if curatedApp}
        <button class="icon-btn" onclick={() => (editing = true)} title="Edit this app" aria-label="Edit this app">
          <Pencil size={16} />
        </button>
      {/if}
    </div>

    {#if !curatedKnown}
      <p class="muted pad">Loading…</p>
    {:else}
      <div class="ident" oncontextmenu={headerMenu} role="group">
        <AppIcon {name} size={64} {source} {id} homepage={iconSource} />
        <h1>{name}</h1>
        <div class="meta">
          {#each variants as v (v.source)}<SourceBadge source={v.source} />{/each}
        </div>
        <div class="sub mono">{id}</div>
        {#if publisher}<div class="sub muted">{publisher}</div>{/if}
        {#if tags.length}
          <div class="det-tags">
            {#each tags as t (t)}
              <button class="det-tag" onclick={() => filterByTag(t)} title={`Filter Discover by "${t}"`}>{t}</button>
            {/each}
          </div>
        {/if}
      </div>

      <div class="actions">
        {#if $installedReady}
          {#if updatable && updateVariant}
            <InstallButton source={updateVariant.source} id={updateVariant.id} {name} kind="update" onDone={onChanged} />
          {/if}
          {#if isInstalled && installedVariant}
            <InstallButton source={installedVariant.source} id={installedVariant.id} {name} kind="uninstall" onDone={onChanged} />
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
          <button class="btn btn-ghost" onclick={() => openUrl(homepage!)}><ExternalLink size={15} /> Website</button>
        {/if}
        {#if donate}
          <button class="btn btn-ghost" onclick={() => openUrl(donate!)}><Heart size={15} /> Donate</button>
        {/if}
        {#if releaseNotes}
          <button class="btn btn-ghost" onclick={() => openUrl(releaseNotes!)}><FileText size={15} /> Release notes</button>
        {/if}
        {#if !curatedApp}
          <button class="btn btn-ghost" onclick={addThis} disabled={adding}>
            <Plus size={15} /> {adding ? 'Adding…' : 'Add to my list'}
          </button>
        {/if}
      </div>
    {/if}
  </div>

  <div class="browse-main">
    {#if editing}
      <CuratedAppEditForm {source} {id} onClose={() => (editing = false)} onSaved={onChanged} />
    {:else if curatedKnown}
      <div class="pane-scroll">
        <div class="facts">
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
          <p class="muted small pad">Loading details…</p>
        {:else}
          <p class="muted small pad">No description available.</p>
        {/if}
      </div>
    {/if}
  </div>
</div>

<style>
  .browse-panel {
    flex: 1;
    min-height: 0;
    display: flex;
    align-items: stretch;
    overflow: hidden;
    background: var(--surface);
  }
  .side {
    flex: 0 0 260px;
    display: flex;
    flex-direction: column;
    overflow-y: auto;
    border-right: 1px solid var(--border);
    background: var(--surface-2);
  }
  .rail-head {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
  }
  .spacer {
    flex: 1;
  }
  .back-btn {
    flex-shrink: 0;
    width: 30px;
    height: 30px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface);
    color: var(--text-muted);
    line-height: 0;
    text-decoration: none;
  }
  .back-btn:hover {
    background: var(--surface-hover);
    color: var(--text);
    border-color: var(--accent);
  }
  .icon-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 30px;
    height: 30px;
    border: 1px solid transparent;
    background: transparent;
    color: var(--text-muted);
    border-radius: var(--radius-sm);
    line-height: 0;
  }
  .icon-btn:hover {
    background: var(--surface-hover);
    color: var(--text);
    border-color: var(--border);
  }
  .ident {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 18px 16px 14px;
  }
  .ident h1 {
    font-size: 1.15rem;
    margin: 4px 0 0;
  }
  .meta {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .sub {
    font-size: 0.76rem;
    color: var(--text-muted);
    word-break: break-all;
  }
  .det-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 5px;
    margin-top: 4px;
  }
  .det-tag {
    font-size: 0.66rem;
    font-family: var(--font-mono);
    color: var(--text-muted);
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 1px 7px;
    cursor: pointer;
    white-space: nowrap;
  }
  .det-tag:hover {
    color: var(--accent);
    border-color: var(--accent);
  }
  .actions {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 0 16px 18px;
  }
  .browse-main {
    flex: 1;
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .pane-scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
  }
  .facts {
    display: flex;
    flex-direction: column;
  }
  .fact {
    display: flex;
    gap: 16px;
    padding: 12px 20px;
    border-top: 1px solid var(--border);
  }
  .fact:first-child {
    border-top: none;
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
  .desc-block {
    border-top: 1px solid var(--border);
    padding: 20px;
  }
  .desc-block h2 {
    font-size: 1.05rem;
    margin-bottom: 8px;
  }
  .desc-block p {
    color: var(--text);
    line-height: 1.6;
    max-width: 72ch;
  }
  .pad {
    padding: 20px;
  }
  .small {
    font-size: 0.85rem;
  }
  @media (max-width: 720px) {
    .browse-panel {
      flex-direction: column;
    }
    .side {
      flex: none;
      border-right: none;
      border-bottom: 1px solid var(--border);
    }
  }
</style>

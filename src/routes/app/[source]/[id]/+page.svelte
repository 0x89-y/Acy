<script lang="ts">
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import {
    ExternalLink,
    ArrowLeft,
    Heart,
    Plus,
    Pencil,
    FileText,
    Copy,
    Check
  } from '@lucide/svelte';
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
  import type { CuratedApp, Package, Source, Variant } from '$lib/types';

  const SOURCE_LABELS: Record<Source, string> = {
    winget: 'winget',
    scoop: 'Scoop',
    choco: 'Chocolatey',
    msstore: 'Microsoft Store'
  };
  const RELATED_MAX = 6;
  // Release notes longer than this start collapsed.
  const NOTES_PREVIEW_LINES = 12;

  let source = $derived($page.params.source as Source);
  let id = $derived($page.params.id ?? '');
  let idLower = $derived(id.toLowerCase());
  let backHref = $derived.by(() => {
    const requested = $page.url.searchParams.get('back');
    if (requested?.startsWith('/') && !requested.startsWith('//')) return requested;
    return $page.url.searchParams.get('from') === 'installed' ? '/?view=library' : '/';
  });

  let curatedHit = $derived.by(() => {
    const c = $curatedStore;
    if (!c) return null;
    for (const cat of c.categories) {
      for (const app of cat.apps) {
        if (app.source === source && app.id.toLowerCase() === idLower) return { app, cat };
      }
    }
    return null;
  });
  let curatedApp = $derived(curatedHit?.app ?? null);
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

  // Catalog fields win (they're curated); the package manager fills the rest.
  let name = $derived(curatedApp?.name ?? info?.name ?? installedPkg?.name ?? id);
  let homepage = $derived(curatedApp?.homepage ?? info?.homepage ?? null);
  let iconSource = $derived(curatedApp?.icon ?? homepage);
  let tagline = $derived(curatedApp?.description ?? null);
  // The package manager's (usually longer) description, unless it just
  // repeats the catalog tagline shown in the header.
  let about = $derived.by(() => {
    const full = info?.description ?? null;
    if (!full || full.trim() === tagline?.trim()) return null;
    return full;
  });
  let tags = $derived(curatedApp?.tags ?? []);
  let donate = $derived(curatedApp?.donate ?? null);
  let releaseNotesUrl = $derived(curatedApp?.releaseNotes ?? info?.releaseNotesUrl ?? null);
  let releaseNotes = $derived(info?.releaseNotes ?? null);
  let publisher = $derived(info?.publisher ?? installedPkg?.publisher ?? null);
  let latestVersion = $derived(updatePkg?.availableVersion ?? info?.version ?? null);
  let isInstalled = $derived(!!installedPkg);
  let updatable = $derived(!!updatePkg);

  let related = $derived.by<CuratedApp[]>(() => {
    const hit = curatedHit;
    if (!hit) return [];
    return hit.cat.apps.filter((a) => a !== hit.app).slice(0, RELATED_MAX);
  });

  let notesExpanded = $state(false);
  let notesLong = $derived((releaseNotes?.split('\n').length ?? 0) > NOTES_PREVIEW_LINES);
  let notesShown = $derived(
    notesLong && !notesExpanded
      ? releaseNotes!.split('\n').slice(0, NOTES_PREVIEW_LINES).join('\n')
      : releaseNotes
  );

  $effect(() => {
    loadCurated();
    loadInstalled();
    loadUpdates();
  });

  // Always ask the package manager: even catalog apps get their publisher,
  // version, licence, full description and release notes from it.
  $effect(() => {
    const s = source;
    const i = id;
    if (!curatedKnown) return;
    loadingInfo = true;
    info = null;
    notesExpanded = false;
    api
      .appInfo(s, i)
      .then((d) => {
        if (source === s && id === i) info = d;
      })
      .catch(() => {})
      .finally(() => {
        if (source === s && id === i) loadingInfo = false;
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
    await addToCurated({ source, id, name, description: tagline ?? about, homepage, alternates });
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

  let copied = $state<string | null>(null);
  async function copy(key: string, text: string) {
    if (await copyText(text)) {
      copied = key;
      setTimeout(() => {
        if (copied === key) copied = null;
      }, 1500);
    }
  }

  function relatedHref(a: CuratedApp) {
    return `/app/${a.source}/${encodeURIComponent(a.id)}?back=${encodeURIComponent(backHref)}`;
  }
</script>

<svelte:window
  onkeydown={(e) => {
    if (e.key !== 'Escape') return;
    if (editing) editing = false;
    else goto(backHref);
  }}
/>

<div class="detail">
  <div class="topbar">
    <a class="back-btn" href={backHref} title="Back" aria-label="Back"><ArrowLeft size={17} /></a>
    <div class="spacer"></div>
    {#if curatedApp && !editing}
      <button class="icon-btn" onclick={() => (editing = true)} title="Edit this app" aria-label="Edit this app">
        <Pencil size={16} />
      </button>
    {/if}
  </div>

  {#if editing}
    <CuratedAppEditForm {source} {id} onClose={() => (editing = false)} onSaved={onChanged} />
  {:else}
  <div class="scroll">
    {#if !curatedKnown}
      <p class="muted column pad">Loading…</p>
    {:else}
      <div class="column">
        <header class="hero" oncontextmenu={headerMenu} role="group">
          <AppIcon {name} size={72} {source} {id} homepage={iconSource} />
          <div class="hero-text">
            <h1>{name}</h1>
            {#if publisher || latestVersion}
              <div class="byline muted">
                {#if publisher}
                  {#if info?.publisherUrl}
                    <button class="link-btn" onclick={() => openUrl(info!.publisherUrl!)}>{publisher}</button>
                  {:else}
                    <span>{publisher}</span>
                  {/if}
                {/if}
                {#if publisher && latestVersion}<span class="dot">·</span>{/if}
                {#if latestVersion}<span class="mono">v{latestVersion}</span>{/if}
              </div>
            {/if}
            {#if tagline}<p class="tagline">{tagline}</p>{/if}
            {#if tags.length}
              <div class="tags">
                {#each tags as t (t)}
                  <button class="tag" onclick={() => filterByTag(t)} title={`Show "${t}" apps in Discover`}>{t}</button>
                {/each}
              </div>
            {/if}
          </div>

          <div class="hero-action">
            {#if $installedReady}
              {#if updatable && updateVariant}
                <InstallButton source={updateVariant.source} id={updateVariant.id} {name} kind="update" onDone={onChanged} />
              {/if}
              {#if isInstalled && installedVariant}
                <InstallButton source={installedVariant.source} id={installedVariant.id} {name} kind="uninstall" onDone={onChanged} />
              {:else if variants.length > 1}
                <InstallSplitButton {variants} {name} preferred={$settings.preferredSource} onDone={onChanged} />
              {:else}
                <InstallButton {source} {id} {name} kind="install" onDone={onChanged} />
              {/if}
              <span class="status muted">
                {#if updatable && updatePkg}
                  Update available
                {:else if isInstalled}
                  <Check size={13} /> Installed{installedPkg?.version ? ` · v${installedPkg.version}` : ''}
                {:else}
                  Not installed
                {/if}
              </span>
            {:else}
              <span class="status muted">Checking status…</span>
            {/if}
          </div>
        </header>

        <div class="links">
          {#if homepage}
            <button class="btn btn-ghost" onclick={() => openUrl(homepage!)}><ExternalLink size={15} /> Website</button>
          {/if}
          {#if releaseNotesUrl}
            <button class="btn btn-ghost" onclick={() => openUrl(releaseNotesUrl!)}><FileText size={15} /> Release notes</button>
          {/if}
          {#if donate}
            <button class="btn btn-ghost" onclick={() => openUrl(donate!)}><Heart size={15} /> Donate</button>
          {/if}
          {#if !curatedApp}
            <button class="btn btn-ghost" onclick={addThis} disabled={adding}>
              <Plus size={15} /> {adding ? 'Adding…' : 'Add to my list'}
            </button>
          {/if}
        </div>

        {#if about}
          <section>
            <h2>About</h2>
            <p class="prose">{about}</p>
          </section>
        {:else if loadingInfo && !tagline}
          <p class="muted small">Loading details…</p>
        {:else if !tagline && !loadingInfo}
          <p class="muted small">No description available.</p>
        {/if}

        {#if releaseNotes}
          <section>
            <h2>What's new{latestVersion ? ` in v${latestVersion}` : ''}</h2>
            <p class="prose notes">{notesShown}</p>
            {#if notesLong}
              <button class="link-btn more" onclick={() => (notesExpanded = !notesExpanded)}>
                {notesExpanded ? 'Show less' : 'Show more'}
              </button>
            {/if}
          </section>
        {/if}

        <section>
          <h2>Details</h2>
          <dl class="facts">
            {#if installedPkg?.version}
              <div><dt>Installed version</dt><dd class="mono">{installedPkg.version}</dd></div>
            {/if}
            {#if latestVersion}
              <div><dt>{updatable ? 'Available version' : 'Latest version'}</dt><dd class="mono">{latestVersion}</dd></div>
            {/if}
            {#if publisher}
              <div><dt>Publisher</dt><dd>{publisher}</dd></div>
            {/if}
            {#if info?.author && info.author !== publisher}
              <div><dt>Author</dt><dd>{info.author}</dd></div>
            {/if}
            {#if info?.license}
              <div>
                <dt>License</dt>
                <dd>
                  {#if info.licenseUrl}
                    <button class="link-btn" onclick={() => openUrl(info!.licenseUrl!)}>{info.license}</button>
                  {:else}
                    {info.license}
                  {/if}
                </dd>
              </div>
            {/if}
            {#if homepage}
              <div>
                <dt>Website</dt>
                <dd><button class="link-btn mono url" onclick={() => openUrl(homepage!)}>{homepage}</button></dd>
              </div>
            {/if}
            {#if installedPkg?.installLocation}
              <div><dt>Location</dt><dd class="mono url">{installedPkg.installLocation}</dd></div>
            {/if}
          </dl>

          <div class="packages">
            {#each variants as v (v.source)}
              {@const vcmd = installCommand(v.source, v.id)}
              <div class="pkg">
                <SourceBadge source={v.source} id={v.id} />
                <span class="pkg-id mono" title={`${SOURCE_LABELS[v.source]} package id`}>{v.id}</span>
                <button
                  class="icon-btn"
                  onclick={() => copy(`id:${v.source}`, v.id)}
                  title="Copy id"
                  aria-label={`Copy ${SOURCE_LABELS[v.source]} id`}
                >
                  {#if copied === `id:${v.source}`}<Check size={14} />{:else}<Copy size={14} />{/if}
                </button>
                {#if vcmd}
                  <button class="btn btn-ghost cmd" onclick={() => copy(`cmd:${v.source}`, vcmd)} title={vcmd}>
                    {copied === `cmd:${v.source}` ? 'Copied' : 'Copy command'}
                  </button>
                {/if}
              </div>
            {/each}
          </div>
        </section>

        {#if related.length}
          <section>
            <h2>More in {curatedHit!.cat.title}</h2>
            <div class="related">
              {#each related as a (a.source + a.id)}
                <a class="rel" href={relatedHref(a)}>
                  <AppIcon name={a.name ?? a.id} size={36} source={a.source} id={a.id} homepage={a.icon ?? a.homepage} />
                  <div class="rel-text">
                    <span class="rel-name">{a.name ?? a.id}</span>
                    {#if a.description}<span class="rel-desc muted">{a.description}</span>{/if}
                  </div>
                </a>
              {/each}
            </div>
          </section>
        {/if}
      </div>
    {/if}
  </div>
  {/if}
</div>

<style>
  .detail {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    background: var(--surface);
  }
  .topbar {
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
    flex-shrink: 0;
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
  .scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
  }
  .column {
    max-width: 880px;
    margin: 0 auto;
    padding: 28px 32px 48px;
  }
  .pad {
    padding-top: 40px;
  }

  .hero {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    gap: 20px;
    align-items: start;
  }
  .hero-text {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .hero h1 {
    font-size: 1.6rem;
    line-height: 1.2;
    margin: 2px 0 0;
  }
  .byline {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    font-size: 0.9rem;
  }
  .dot {
    opacity: 0.6;
  }
  .tagline {
    margin: 4px 0 0;
    line-height: 1.5;
    max-width: 60ch;
  }
  .tags {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: 6px;
  }
  .tag {
    font-size: 0.72rem;
    color: var(--text-muted);
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 2px 10px;
    cursor: pointer;
  }
  .tag:hover {
    color: var(--accent);
    border-color: var(--accent);
  }
  .hero-action {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: 8px;
    min-width: 150px;
  }
  .status {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 4px;
    font-size: 0.8rem;
  }
  .status :global(svg) {
    color: var(--success);
  }

  .links {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin: 18px 0 0 -10px;
  }

  section {
    margin-top: 28px;
    padding-top: 24px;
    border-top: 1px solid var(--border);
  }
  section h2 {
    font-size: 1.02rem;
    margin: 0 0 12px;
  }
  .prose {
    margin: 0;
    line-height: 1.65;
    max-width: 72ch;
    white-space: pre-line;
  }
  .notes {
    font-size: 0.9rem;
    color: var(--text-muted);
  }
  .more {
    margin-top: 8px;
  }

  .facts {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 14px 28px;
    margin: 0;
  }
  .facts div {
    min-width: 0;
  }
  .facts dt {
    font-size: 0.78rem;
    color: var(--text-muted);
    margin-bottom: 2px;
  }
  .facts dd {
    margin: 0;
    font-size: 0.92rem;
    overflow-wrap: anywhere;
  }
  .url {
    font-size: 0.82rem;
  }

  .packages {
    display: flex;
    flex-direction: column;
    margin-top: 18px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }
  .pkg {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 6px 8px 6px 12px;
  }
  .pkg + .pkg {
    border-top: 1px solid var(--border);
  }
  .pkg-id {
    flex: 1;
    min-width: 0;
    font-size: 0.82rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .cmd {
    font-size: 0.8rem;
    padding: 5px 10px;
  }

  .related {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
    gap: 8px;
  }
  .rel {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px;
    border-radius: var(--radius);
    color: inherit;
    text-decoration: none;
    min-width: 0;
  }
  .rel:hover {
    background: var(--surface-hover);
  }
  .rel-text {
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .rel-name {
    font-weight: 600;
    font-size: 0.9rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .rel-desc {
    font-size: 0.78rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .link-btn {
    background: none;
    border: none;
    padding: 0;
    color: var(--accent);
    cursor: pointer;
    font: inherit;
    text-align: left;
  }
  .link-btn:hover {
    text-decoration: underline;
  }
  .small {
    font-size: 0.85rem;
    margin-top: 24px;
  }

  @media (max-width: 760px) {
    .hero {
      grid-template-columns: auto minmax(0, 1fr);
    }
    .hero-action {
      grid-column: 1 / -1;
      flex-direction: row;
      align-items: center;
    }
    .facts {
      grid-template-columns: minmax(0, 1fr);
    }
  }
</style>

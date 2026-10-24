<script lang="ts">
  import { onMount } from 'svelte';
  import {
    ArrowLeft,
    ArrowUp,
    ArrowDown,
    Trash2,
    Plus,
    ChevronRight,
    ChevronDown
  } from '@lucide/svelte';
  import SourceBadge from '$lib/components/SourceBadge.svelte';
  import AppIconControls from '$lib/components/AppIconControls.svelte';
  import * as api from '$lib/api';
  import { settings } from '$lib/stores/settings';
  import type { CuratedFile, CuratedApp, CuratedCategory, Source } from '$lib/types';

  const sources: Source[] = ['winget', 'scoop', 'choco', 'msstore'];

  let file = $state<CuratedFile | null>(null);
  let loading = $state(true);
  let saving = $state(false);
  let status = $state<{ kind: 'ok' | 'err'; msg: string } | null>(null);

  let selectedCat = $state<number | 'all'>('all');
  let activeCat = $derived(
    typeof selectedCat === 'number' ? (file?.categories[selectedCat] ?? null) : null
  );
  let allApps = $derived.by(() => {
    const out: { cat: CuratedCategory; ci: number; app: CuratedApp; ai: number }[] = [];
    file?.categories.forEach((cat, ci) => cat.apps.forEach((app, ai) => out.push({ cat, ci, app, ai })));
    return out;
  });
  $effect(() => {
    const n = file?.categories.length ?? 0;
    if (typeof selectedCat === 'number' && selectedCat > n - 1) selectedCat = 'all';
  });

  let filter = $state('');
  let filtering = $derived(filter.trim().length > 0);
  function matchesApp(app: CuratedApp): boolean {
    const q = filter.trim().toLowerCase();
    if (!q) return true;
    const hay = [app.name ?? '', app.id, app.source, ...(app.tags ?? []), ...app.alternates.map((a) => a.id)]
      .join(' ')
      .toLowerCase();
    return hay.includes(q);
  }
  function catMatches(cat: CuratedCategory): boolean {
    const q = filter.trim().toLowerCase();
    if (!q) return true;
    return `${cat.title} ${cat.id}`.toLowerCase().includes(q);
  }
  const catShown = (cat: CuratedCategory) => catMatches(cat) || cat.apps.some(matchesApp);
  const appShown = (cat: CuratedCategory, app: CuratedApp) => catMatches(cat) || matchesApp(app);

  let hideBuiltins = $derived(!$settings.showCuratedApps);
  const appVisible = (cat: CuratedCategory, app: CuratedApp) =>
    (!filtering || appShown(cat, app)) && (!hideBuiltins || app.custom);
  const catVisible = (cat: CuratedCategory) =>
    (!filtering || catShown(cat)) && (!hideBuiltins || cat.apps.some((a) => a.custom));
  const shownCount = (cat: CuratedCategory) =>
    hideBuiltins ? cat.apps.filter((a) => a.custom).length : cat.apps.length;

  let openApp = $state<Record<string, boolean>>({});
  const appOpen = (ci: number, ai: number) => openApp[`${ci}-${ai}`] ?? false;

  onMount(async () => {
    try {
      file = await api.getCurated();
    } catch (e) {
      status = { kind: 'err', msg: `Could not load catalog: ${e}` };
    } finally {
      loading = false;
    }
  });

  function move<T>(arr: T[], i: number, delta: number) {
    const j = i + delta;
    if (j < 0 || j >= arr.length) return;
    [arr[i], arr[j]] = [arr[j], arr[i]];
  }

  function addCategory() {
    if (!file) return;
    file.categories.push({ id: '', title: '', apps: [] });
    selectedCat = file.categories.length - 1;
  }
  function removeCategory(ci: number) {
    file?.categories.splice(ci, 1);
  }
  function moveCategory(ci: number, delta: number) {
    if (!file) return;
    move(file.categories, ci, delta);
    if (selectedCat === ci) selectedCat = Math.max(0, Math.min(file.categories.length - 1, ci + delta));
  }
  function addApp(cat: CuratedCategory, ci: number) {
    cat.apps.push({
      id: '',
      source: 'winget',
      name: null,
      description: null,
      homepage: null,
      icon: null,
      alternates: [],
      tags: [],
      donate: null,
      releaseNotes: null,
      custom: true
    });
    openApp[`${ci}-${cat.apps.length - 1}`] = true;
  }
  function addAlternate(app: CuratedApp) {
    const used = new Set<Source>([app.source, ...app.alternates.map((v) => v.source)]);
    const next = sources.find((s) => !used.has(s));
    if (next) app.alternates.push({ source: next, id: '' });
  }
  function removeAlternate(app: CuratedApp, i: number) {
    app.alternates.splice(i, 1);
  }
  function removePrimary(app: CuratedApp) {
    const next = app.alternates.shift();
    if (next) {
      app.source = next.source;
      app.id = next.id;
    }
  }
  function removeApp(cat: CuratedCategory, ai: number) {
    cat.apps.splice(ai, 1);
  }

  function clean(file: CuratedFile): CuratedFile {
    const blank = (v: string | null) => (v && v.trim() ? v.trim() : null);
    return {
      version: file.version,
      categories: file.categories.map((c) => ({
        id: c.id.trim(),
        title: c.title.trim(),
        apps: c.apps.map((a) => {
          const seen = new Set<Source>();
          const uniq = [{ source: a.source, id: a.id }, ...a.alternates]
            .map((v) => ({ source: v.source, id: v.id.trim() }))
            .filter((v) => v.id && !seen.has(v.source) && seen.add(v.source));
          const primary = uniq[0] ?? { source: a.source, id: '' };
          return {
            id: primary.id,
            source: primary.source,
            name: blank(a.name),
            description: blank(a.description),
            homepage: blank(a.homepage),
            icon: blank(a.icon),
            alternates: uniq.slice(1),
            tags: (a.tags ?? []).map((t) => t.trim()).filter(Boolean),
            donate: blank(a.donate),
            releaseNotes: blank(a.releaseNotes),
            custom: a.custom
          };
        })
      }))
    };
  }

  function validate(f: CuratedFile): string | null {
    for (const c of f.categories) {
      if (!c.id || !c.title) return 'Every category needs an id and a title.';
      for (const a of c.apps) {
        if (!a.id) return `An app in "${c.title}" is missing its package id.`;
      }
    }
    return null;
  }

  async function save() {
    if (!file) return;
    const cleaned = clean(file);
    const err = validate(cleaned);
    if (err) {
      status = { kind: 'err', msg: err };
      return;
    }
    saving = true;
    status = null;
    try {
      await api.saveCurated(cleaned);
      file = cleaned;
      status = { kind: 'ok', msg: 'Saved. Changes appear on the Discover page.' };
    } catch (e) {
      status = { kind: 'err', msg: `Save failed: ${e}` };
    } finally {
      saving = false;
    }
  }

  let appCount = $derived(file?.categories.reduce((n, c) => n + shownCount(c), 0) ?? 0);
</script>

{#snippet appRow(cat: CuratedCategory, ci: number, app: CuratedApp, ai: number, showCat: boolean)}
  <div class="app" class:open={appOpen(ci, ai)}>
    <div class="app-row">
      <button class="disclose" onclick={() => (openApp[`${ci}-${ai}`] = !appOpen(ci, ai))} aria-label="Toggle app">
        {#if appOpen(ci, ai)}<ChevronDown size={15} />{:else}<ChevronRight size={15} />{/if}
      </button>
      <button class="app-name" onclick={() => (openApp[`${ci}-${ai}`] = !appOpen(ci, ai))}>
        {app.name || app.id || 'Untitled app'}
      </button>
      {#if showCat}<span class="app-cat">{cat.title || cat.id || '-'}</span>{/if}
      {#if app.custom}<span class="tag">custom</span>{/if}
      <div class="app-sources">
        {#if app.id || app.alternates.length}<SourceBadge source={app.source} />{/if}
        {#each app.alternates as alt, k (k)}<SourceBadge source={alt.source} />{/each}
      </div>
      <div class="row-actions">
        <button class="icon-btn" title="Move up" onclick={() => move(cat.apps, ai, -1)}>
          <ArrowUp size={14} />
        </button>
        <button class="icon-btn" title="Move down" onclick={() => move(cat.apps, ai, 1)}>
          <ArrowDown size={14} />
        </button>
        <button class="icon-btn danger" title="Remove app" onclick={() => removeApp(cat, ai)}>
          <Trash2 size={14} />
        </button>
      </div>
    </div>

    {#if appOpen(ci, ai)}
      <div class="app-edit">
        <div class="edit-list">
          <div class="edit-row">
            <span class="fl">Icon</span>
            <AppIconControls
              source={app.source}
              id={app.id}
              name={app.name ?? ''}
              from={app.icon || app.homepage || null}
              size={40}
            >
              <input
                class="in"
                placeholder="image or page URL - defaults to the homepage"
                bind:value={app.icon}
              />
            </AppIconControls>
          </div>
          <label class="edit-row">
            <span class="fl">Name</span>
            <input class="in" placeholder="optional" bind:value={app.name} />
          </label>
          <label class="edit-row">
            <span class="fl">Description</span>
            <input class="in" placeholder="optional" bind:value={app.description} />
          </label>
          <label class="edit-row">
            <span class="fl">Tags</span>
            <input
              class="in"
              placeholder="open source, free, chromium"
              value={(app.tags ?? []).join(', ')}
              onchange={(e) => (app.tags = e.currentTarget.value.split(',').map((t) => t.trim()).filter(Boolean))}
            />
          </label>
          <label class="edit-row">
            <span class="fl">Homepage</span>
            <input class="in" placeholder="optional" bind:value={app.homepage} />
          </label>
          <label class="edit-row">
            <span class="fl">Donate</span>
            <input class="in" placeholder="optional" bind:value={app.donate} />
          </label>
          <label class="edit-row">
            <span class="fl">Release notes</span>
            <input class="in" placeholder="optional" bind:value={app.releaseNotes} />
          </label>

          <div class="edit-row srcs">
            <span class="fl">Sources</span>
            <div class="src-list">
              <div class="src-row">
                <select class="src-pick {app.source}" bind:value={app.source} title="Package manager">
                  {#each sources as s (s)}<option value={s}>{s}</option>{/each}
                </select>
                <input class="src-id mono" placeholder="package id" bind:value={app.id} />
                <button
                  class="icon-btn danger row-del"
                  title="Remove source"
                  aria-label="Remove source"
                  disabled={app.alternates.length === 0}
                  onclick={() => removePrimary(app)}
                >
                  <Trash2 size={14} />
                </button>
              </div>
              {#each app.alternates as alt, k (k)}
                <div class="src-row">
                  <select class="src-pick {alt.source}" bind:value={alt.source} title="Package manager">
                    {#each sources as s (s)}<option value={s}>{s}</option>{/each}
                  </select>
                  <input class="src-id mono" placeholder="package id" bind:value={alt.id} />
                  <button class="icon-btn danger row-del" title="Remove source" aria-label="Remove source" onclick={() => removeAlternate(app, k)}>
                    <Trash2 size={14} />
                  </button>
                </div>
              {/each}
              <button class="btn btn-ghost add-src" onclick={() => addAlternate(app)}>
                <Plus size={14} /> Add source
              </button>
            </div>
          </div>
        </div>
      </div>
    {/if}
  </div>
{/snippet}

<div class="screen">
  {#if loading}
    <p class="pane-msg muted">Loading…</p>
  {:else if file}
    <div class="browse-panel">
      <div class="browse-rail">
        <div class="rail-head">
          <a class="back-btn" href="/settings" title="Back" aria-label="Back"><ArrowLeft size={17} /></a>
          <span class="rail-title">Curated catalog</span>
        </div>
        <div class="rail-tools">
          <input class="head-filter" placeholder="Search apps…" bind:value={filter} />
          <button class="btn btn-accent save-btn" onclick={save} disabled={saving || !file}>
            {saving ? 'Saving…' : 'Save'}
          </button>
        </div>
        {#if status}
          <p class="status" class:ok={status.kind === 'ok'} class:err={status.kind === 'err'}>{status.msg}</p>
        {/if}
        <div class="rail-links">
          <button class="rail-link" class:active={selectedCat === 'all'} onclick={() => (selectedCat = 'all')}>
            <span>All apps</span><span class="rail-count mono">{appCount}</span>
          </button>
          {#each file.categories as cat, ci (ci)}
            {#if catVisible(cat)}
              <button class="rail-link" class:active={selectedCat === ci} onclick={() => (selectedCat = ci)}>
                <span>{cat.title || cat.id || 'Untitled'}</span>
                <span class="rail-count mono">{shownCount(cat)}</span>
              </button>
            {/if}
          {/each}
          <button class="rail-link add-rail" onclick={addCategory}>
            <Plus size={15} /> Add category
          </button>
        </div>
      </div>

      <div class="browse-main">
        {#if selectedCat === 'all'}
          <div class="pane-head">
            <span class="pane-title">All apps</span>
            <span class="rail-count mono">{appCount}</span>
          </div>
          <div class="pane-scroll">
            <div class="apps">
              {#each allApps as e (e.ci + '-' + e.ai)}
                {#if appVisible(e.cat, e.app)}
                  {@render appRow(e.cat, e.ci, e.app, e.ai, true)}
                {/if}
              {/each}
            </div>
          </div>
        {:else if activeCat}
          <div class="pane-head">
            <input class="in title" placeholder="Category title" bind:value={activeCat.title} />
            <input class="in id" placeholder="id (slug)" bind:value={activeCat.id} />
            <div class="spacer"></div>
            <button class="icon-btn" title="Move up" onclick={() => moveCategory(selectedCat as number, -1)}>
              <ArrowUp size={15} />
            </button>
            <button class="icon-btn" title="Move down" onclick={() => moveCategory(selectedCat as number, 1)}>
              <ArrowDown size={15} />
            </button>
            <button class="icon-btn danger" title="Remove category" onclick={() => removeCategory(selectedCat as number)}>
              <Trash2 size={15} />
            </button>
          </div>
          <div class="pane-scroll">
            <div class="apps">
              {#each activeCat.apps as app, ai (ai)}
                {#if appVisible(activeCat, app)}
                  {@render appRow(activeCat, selectedCat as number, app, ai, false)}
                {/if}
              {/each}
            </div>
            <button class="btn btn-ghost add" onclick={() => activeCat && addApp(activeCat, selectedCat as number)}>
              <Plus size={15} /> Add app
            </button>
          </div>
        {:else}
          <p class="pane-msg muted">No categories yet - add one on the left.</p>
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .screen {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .rail-head {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
  }
  .rail-title {
    font-size: 0.95rem;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .rail-tools {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
  }
  .head-filter {
    flex: 1;
    min-width: 0;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    color: var(--text);
    padding: 6px 10px;
    font-size: 0.85rem;
    outline: none;
  }
  .head-filter:focus {
    border-color: var(--accent);
  }
  .save-btn {
    flex-shrink: 0;
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
  .spacer {
    flex: 1;
  }
  .status {
    flex-shrink: 0;
    font-size: 0.78rem;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
  }
  .status.ok {
    color: var(--success);
    background: color-mix(in srgb, var(--success) 12%, transparent);
  }
  .status.err {
    color: var(--danger);
    background: color-mix(in srgb, var(--danger) 12%, transparent);
  }
  .pane-msg {
    padding: 24px 20px;
  }

  .browse-panel {
    flex: 1;
    min-height: 0;
    display: flex;
    align-items: stretch;
    overflow: hidden;
    background: var(--surface);
  }
  .browse-rail {
    flex: 0 0 190px;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    border-right: 1px solid var(--border);
    background: var(--surface-2);
  }
  .rail-links {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
  }
  .rail-link {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    text-align: left;
    padding: 9px 14px;
    border: none;
    border-top: 1px solid var(--border);
    border-left: 2px solid transparent;
    border-radius: 0;
    background: transparent;
    color: var(--text-muted);
    font-size: 0.9rem;
    font-weight: 500;
  }
  .rail-link:first-child {
    border-top: none;
  }
  .rail-link span:first-child {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .rail-link:hover {
    background: var(--surface-hover);
    color: var(--text);
  }
  .rail-link.active {
    background: var(--surface);
    color: var(--text);
    border-left-color: var(--accent);
  }
  .rail-count {
    font-size: 0.72rem;
    color: var(--text-muted);
  }
  .rail-link.active .rail-count {
    color: var(--accent);
  }
  .add-rail {
    margin-top: auto;
    color: var(--accent);
    gap: 6px;
    justify-content: flex-start;
  }
  .browse-main {
    flex: 1;
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .pane-head {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 34px;
    padding: 10px 14px;
    border-bottom: 1px solid var(--border);
  }
  .pane-scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
  }

  .apps {
    display: flex;
    flex-direction: column;
  }
  .app {
    border-top: 1px solid var(--border);
  }
  .app:first-child {
    border-top: none;
  }
  .app.open {
    background: var(--surface-2);
  }
  .app-cat {
    font-size: 0.72rem;
    color: var(--text-muted);
    flex-shrink: 0;
    white-space: nowrap;
  }
  .pane-title {
    font-size: 0.95rem;
    font-weight: 600;
  }
  .app-row {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 38px;
    padding: 4px 14px 4px 4px;
  }
  .app-name {
    flex: 1;
    min-width: 0;
    text-align: left;
    background: none;
    border: none;
    color: var(--text);
    font-size: 0.9rem;
    font-weight: 500;
    padding: 2px 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .app-name:hover {
    color: var(--accent);
  }
  .app-sources {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-shrink: 0;
  }
  .tag {
    flex-shrink: 0;
    font-family: var(--font-mono);
    font-size: 0.64rem;
    padding: 1px 7px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--accent);
    color: var(--accent);
    white-space: nowrap;
  }

  .app-edit {
    --field-label: 132px;
    padding: 2px 0 6px 34px;
  }
  .edit-list {
    display: flex;
    flex-direction: column;
    border-top: 1px solid var(--border);
  }
  .edit-row {
    display: grid;
    grid-template-columns: var(--field-label) minmax(0, 1fr);
    align-items: center;
    gap: 14px;
    padding: 9px 14px 9px 0;
  }
  .edit-row + .edit-row {
    border-top: 1px solid var(--border);
  }
  .edit-row.srcs {
    align-items: start;
    padding-top: 11px;
  }
  .edit-row.srcs .fl {
    padding-top: 8px;
  }
  .fl {
    font-size: 0.82rem;
    font-weight: 500;
    color: var(--text-muted);
    min-width: 0;
  }

  .src-list {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .src-row {
    display: grid;
    grid-template-columns: 96px minmax(0, 1fr) auto;
    align-items: center;
    gap: 10px;
    padding: 5px 0;
  }
  .src-row + .src-row {
    border-top: 1px solid var(--border);
  }
  .src-pick {
    appearance: none;
    justify-self: start;
    font-family: var(--font-mono);
    font-size: 0.68rem;
    line-height: 1.5;
    padding: 1px 7px;
    border-radius: var(--radius-sm);
    border: 1px solid currentColor;
    background: transparent;
    color: var(--local);
    outline: none;
  }
  .src-pick:hover {
    background: color-mix(in srgb, currentColor 12%, transparent);
  }
  .src-pick:focus-visible {
    box-shadow: 0 0 0 2px color-mix(in srgb, currentColor 35%, transparent);
  }
  .src-pick option {
    background: var(--surface);
    color: var(--text);
    font-family: var(--font-sans);
  }
  .src-pick.winget {
    color: var(--winget);
  }
  .src-pick.scoop {
    color: var(--scoop);
  }
  .src-pick.choco {
    color: var(--choco);
  }
  .src-pick.msstore {
    color: var(--msstore);
  }
  .src-id {
    min-width: 0;
    width: 100%;
    padding: 5px 8px;
    font-size: 0.8rem;
    color: var(--text);
    background: transparent;
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    outline: none;
  }
  .src-id:hover {
    border-color: var(--border);
  }
  .src-id:focus {
    background: var(--surface);
    border-color: var(--accent);
  }
  .row-del {
    opacity: 0;
    transition: opacity 0.12s;
  }
  .src-row:hover .row-del,
  .src-row:focus-within .row-del {
    opacity: 1;
  }
  .row-del:disabled {
    opacity: 0;
  }
  .add-src {
    align-self: flex-start;
    font-size: 0.82rem;
    padding: 5px 9px;
  }
  .in {
    background: transparent;
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    color: var(--text);
    padding: 6px 9px;
    font-size: 0.86rem;
    outline: none;
    min-width: 0;
    width: 100%;
  }
  .in:hover:not(:disabled) {
    border-color: var(--border);
  }
  .in:focus {
    background: var(--surface);
    border-color: var(--accent);
  }
  .pane-head .in {
    width: auto;
    background: var(--surface);
    border-color: var(--border);
  }
  .title {
    flex: 1;
    font-weight: 600;
  }
  .id {
    flex: 0 0 150px;
    font-family: var(--font-mono);
    font-size: 0.8rem;
  }
  .mono {
    font-family: var(--font-mono);
  }

  .disclose {
    display: inline-flex;
    padding: 4px;
    border: none;
    background: transparent;
    color: var(--text-muted);
    border-radius: var(--radius-sm);
    line-height: 0;
    flex-shrink: 0;
  }
  .disclose:hover {
    color: var(--text);
  }
  .row-actions {
    display: flex;
    gap: 2px;
    flex-shrink: 0;
  }
  .icon-btn {
    display: inline-flex;
    padding: 6px;
    border: none;
    background: transparent;
    color: var(--text-muted);
    border-radius: var(--radius-sm);
    line-height: 0;
  }
  .icon-btn:hover {
    background: var(--surface-hover);
    color: var(--text);
  }
  .icon-btn.danger:hover {
    color: var(--danger);
  }
  .icon-btn:disabled {
    opacity: 0.35;
    cursor: default;
  }
  .icon-btn:disabled:hover {
    background: transparent;
    color: var(--text-muted);
  }

  .add {
    margin: 12px 14px;
    font-size: 0.85rem;
  }
</style>

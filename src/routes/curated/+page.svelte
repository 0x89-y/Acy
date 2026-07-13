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
  import * as api from '$lib/api';
  import type { CuratedFile, CuratedApp, CuratedCategory, Source } from '$lib/types';

  const sources: Source[] = ['winget', 'scoop', 'choco', 'msstore', 'local'];

  async function browse(set: (path: string) => void) {
    const path = await api.pickInstaller();
    if (path) set(path);
  }

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
    const next = sources.find((s) => !used.has(s)) ?? 'local';
    app.alternates.push({ source: next, id: '' });
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
            .filter((v) => (v.id || v.source === 'local') && !seen.has(v.source) && seen.add(v.source));
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
        if (!a.id && a.source !== 'local') return `An app in "${c.title}" is missing its package id.`;
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

  let appCount = $derived(file?.categories.reduce((n, c) => n + c.apps.length, 0) ?? 0);
</script>

{#snippet appRow(cat: CuratedCategory, ci: number, app: CuratedApp, ai: number, showCat: boolean)}
  <div class="app" class:open={appOpen(ci, ai)}>
    <div class="app-row">
      <button class="disclose" onclick={() => (openApp[`${ci}-${ai}`] = !appOpen(ci, ai))} aria-label="Toggle app">
        {#if appOpen(ci, ai)}<ChevronDown size={15} />{:else}<ChevronRight size={15} />{/if}
      </button>
      <span class="tag" class:custom={app.custom}>{app.custom ? 'custom' : 'built-in'}</span>
      <button class="app-name" onclick={() => (openApp[`${ci}-${ai}`] = !appOpen(ci, ai))}>
        {app.name || app.id || 'Untitled app'}
      </button>
      {#if showCat}<span class="app-cat">{cat.title || cat.id || '—'}</span>{/if}
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
        <div class="src-block">
          <span class="fl">Sources</span>
          <div class="src-list">
            <div class="src-row">
              <select class="in src" bind:value={app.source}>
                {#each sources as s (s)}<option value={s}>{s}</option>{/each}
              </select>
              <input
                class="in mono"
                placeholder={app.source === 'local' ? 'installer path (.exe / .msi) — optional' : 'package id for this manager'}
                bind:value={app.id}
              />
              {#if app.source === 'local'}
                <button class="btn btn-ghost browse" onclick={() => browse((p) => (app.id = p))}>Browse…</button>
              {/if}
              <button
                class="icon-btn danger"
                title="Remove source"
                disabled={app.alternates.length === 0}
                onclick={() => removePrimary(app)}
              >
                <Trash2 size={14} />
              </button>
            </div>
            {#each app.alternates as alt, k (k)}
              <div class="src-row">
                <select class="in src" bind:value={alt.source}>
                  {#each sources as s (s)}<option value={s}>{s}</option>{/each}
                </select>
                <input
                  class="in mono"
                  placeholder={alt.source === 'local' ? 'installer path (.exe / .msi) — optional' : 'package id for this manager'}
                  bind:value={alt.id}
                />
                {#if alt.source === 'local'}
                  <button class="btn btn-ghost browse" onclick={() => browse((p) => (alt.id = p))}>Browse…</button>
                {/if}
                <button class="icon-btn danger" title="Remove source" onclick={() => removeAlternate(app, k)}>
                  <Trash2 size={14} />
                </button>
              </div>
            {/each}
            <button class="btn btn-ghost add-src" onclick={() => addAlternate(app)}>
              <Plus size={14} /> Add source
            </button>
          </div>
        </div>

        <div class="fields three">
          <label class="f">
            <span class="fl">Name</span>
            <input class="in" placeholder="optional" bind:value={app.name} />
          </label>
          <label class="f">
            <span class="fl">Homepage</span>
            <input class="in" placeholder="optional" bind:value={app.homepage} />
          </label>
          <label class="f">
            <span class="fl">Icon URL</span>
            <input class="in" placeholder="optional" bind:value={app.icon} />
          </label>
        </div>
        <label class="f">
          <span class="fl">Description</span>
          <input class="in" placeholder="optional" bind:value={app.description} />
        </label>
        <div class="fields two">
          <label class="f">
            <span class="fl">Tags (comma-separated)</span>
            <input
              class="in"
              placeholder="open source, free, chromium"
              value={(app.tags ?? []).join(', ')}
              onchange={(e) => (app.tags = e.currentTarget.value.split(',').map((t) => t.trim()).filter(Boolean))}
            />
          </label>
          <label class="f">
            <span class="fl">Donate URL</span>
            <input class="in" placeholder="optional" bind:value={app.donate} />
          </label>
        </div>
        <label class="f">
          <span class="fl">Release notes URL</span>
          <input class="in" placeholder="optional" bind:value={app.releaseNotes} />
        </label>
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
            {#if !filtering || catShown(cat)}
              <button class="rail-link" class:active={selectedCat === ci} onclick={() => (selectedCat = ci)}>
                <span>{cat.title || cat.id || 'Untitled'}</span>
                <span class="rail-count mono">{cat.apps.length}</span>
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
                {#if !filtering || appShown(e.cat, e.app)}
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
                {#if !filtering || appShown(activeCat, app)}
                  {@render appRow(activeCat, selectedCat as number, app, ai, false)}
                {/if}
              {/each}
            </div>
            <button class="btn btn-ghost add" onclick={() => activeCat && addApp(activeCat, selectedCat as number)}>
              <Plus size={15} /> Add app
            </button>
          </div>
        {:else}
          <p class="pane-msg muted">No categories yet — add one on the left.</p>
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
    flex: 0 0 200px;
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
    gap: 8px;
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
    gap: 8px;
    padding: 6px 8px;
  }
  .app-name {
    flex: 1;
    min-width: 0;
    text-align: left;
    background: none;
    border: none;
    color: var(--text);
    font-size: 0.88rem;
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
    border: 1px solid var(--border-strong);
    color: var(--text-muted);
    white-space: nowrap;
  }
  .tag.custom {
    color: var(--accent);
    border-color: var(--accent);
  }

  .app-edit {
    padding: 4px 10px 12px 34px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .fields {
    display: grid;
    gap: 8px;
  }
  .fields.three {
    grid-template-columns: 1.3fr 0.8fr 1fr;
  }
  .fields.two {
    grid-template-columns: 1.4fr 1fr;
  }
  .f {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
  }
  .fl {
    font-size: 0.72rem;
    color: var(--text-muted);
  }

  .src-block {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .src-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .src-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .add-src {
    align-self: flex-start;
    font-size: 0.8rem;
    padding: 4px 8px;
  }
  .browse {
    flex-shrink: 0;
    font-size: 0.8rem;
    padding: 6px 10px;
    white-space: nowrap;
  }

  .in {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    color: var(--text);
    padding: 7px 10px;
    font-size: 0.86rem;
    outline: none;
    min-width: 0;
    width: 100%;
  }
  .in:focus {
    border-color: var(--accent);
  }
  .in.mono {
    font-family: var(--font-mono);
    font-size: 0.8rem;
  }
  .pane-head .in {
    width: auto;
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
  .src-row .src {
    flex: 0 0 120px;
    width: auto;
  }
  .src-row .mono {
    flex: 1;
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

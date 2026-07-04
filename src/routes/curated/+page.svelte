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

  // Search: hides non-matching apps (and empty categories) without touching the
  // underlying arrays, so the index-based edit/move/remove actions stay correct.
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
  // A category is shown if its title/id matches (then all its apps show) or any
  // of its apps matches.
  const catShown = (cat: CuratedCategory) => catMatches(cat) || cat.apps.some(matchesApp);
  const appShown = (cat: CuratedCategory, app: CuratedApp) => catMatches(cat) || matchesApp(app);

  // Expansion state. Categories default open; apps default collapsed so the page
  // reads as a tidy outline you expand only to edit.
  let openCat = $state<Record<number, boolean>>({});
  let openApp = $state<Record<string, boolean>>({});
  const catOpen = (ci: number) => openCat[ci] ?? true;
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
    file?.categories.push({ id: '', title: '', apps: [] });
  }
  function removeCategory(ci: number) {
    file?.categories.splice(ci, 1);
  }
  function moveCategory(ci: number, delta: number) {
    if (file) move(file.categories, ci, delta);
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
  // The first source row is the primary (app.source/app.id); removing it promotes
  // the next one so the unified list has no visible "main" entry.
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

  // Normalize empty strings back to null so the saved JSON stays clean.
  function clean(file: CuratedFile): CuratedFile {
    const blank = (v: string | null) => (v && v.trim() ? v.trim() : null);
    return {
      version: file.version,
      categories: file.categories.map((c) => ({
        id: c.id.trim(),
        title: c.title.trim(),
        apps: c.apps.map((a) => {
          // Treat primary + alternates as one source list: drop blanks, de-dupe
          // by manager, then the first becomes the primary again.
          const seen = new Set<Source>();
          const uniq = [{ source: a.source, id: a.id }, ...a.alternates]
            .map((v) => ({ source: v.source, id: v.id.trim() }))
            // Keep sources with an id; a local source may have an empty path
            // (the user picks a file at install time).
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

<div class="head">
  <a class="btn btn-ghost" href="/settings"><ArrowLeft size={16} /> Settings</a>
  <h1>Curated catalog</h1>
  <div class="spacer"></div>
  <button class="btn btn-accent" onclick={save} disabled={saving || !file}>
    {saving ? 'Saving…' : 'Save'}
  </button>
</div>

{#if status}
  <p class="status" class:ok={status.kind === 'ok'} class:err={status.kind === 'err'}>{status.msg}</p>
{/if}

{#if loading}
  <p class="muted">Loading…</p>
{:else if file}
  <div class="toolbar">
    <p class="muted count">{file.categories.length} categories · {appCount} apps</p>
    <input class="in search" placeholder="Search apps…" bind:value={filter} />
  </div>

  {#each file.categories as cat, ci (ci)}
    {#if !filtering || catShown(cat)}
    <section class="cat card">
      <div class="cat-head">
        <button class="disclose" onclick={() => (openCat[ci] = !catOpen(ci))} aria-label="Toggle category">
          {#if catOpen(ci)}<ChevronDown size={16} />{:else}<ChevronRight size={16} />{/if}
        </button>
        <input class="in title" placeholder="Category title" bind:value={cat.title} />
        <input class="in id" placeholder="id (slug)" bind:value={cat.id} />
        <span class="chip">{cat.apps.length}</span>
        <div class="row-actions">
          <button class="icon-btn" title="Move up" onclick={() => moveCategory(ci, -1)}>
            <ArrowUp size={15} />
          </button>
          <button class="icon-btn" title="Move down" onclick={() => moveCategory(ci, 1)}>
            <ArrowDown size={15} />
          </button>
          <button class="icon-btn danger" title="Remove category" onclick={() => removeCategory(ci)}>
            <Trash2 size={15} />
          </button>
        </div>
      </div>

      {#if catOpen(ci) || filtering}
        <div class="apps">
          {#each cat.apps as app, ai (ai)}
            {#if !filtering || appShown(cat, app)}
            <div class="app" class:open={appOpen(ci, ai)}>
              <div class="app-row">
                <button
                  class="disclose"
                  onclick={() => (openApp[`${ci}-${ai}`] = !appOpen(ci, ai))}
                  aria-label="Toggle app"
                >
                  {#if appOpen(ci, ai)}<ChevronDown size={15} />{:else}<ChevronRight size={15} />{/if}
                </button>
                <span class="tag" class:custom={app.custom}>{app.custom ? 'custom' : 'built-in'}</span>
                <button
                  class="app-name"
                  onclick={() => (openApp[`${ci}-${ai}`] = !appOpen(ci, ai))}
                >
                  {app.name || app.id || 'Untitled app'}
                </button>
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
                          placeholder={app.source === 'local'
                            ? 'installer path (.exe / .msi) — optional'
                            : 'package id for this manager'}
                          bind:value={app.id}
                        />
                        {#if app.source === 'local'}
                          <button class="btn btn-ghost browse" onclick={() => browse((p) => (app.id = p))}>
                            Browse…
                          </button>
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
                            placeholder={alt.source === 'local'
                              ? 'installer path (.exe / .msi) — optional'
                              : 'package id for this manager'}
                            bind:value={alt.id}
                          />
                          {#if alt.source === 'local'}
                            <button class="btn btn-ghost browse" onclick={() => browse((p) => (alt.id = p))}>
                              Browse…
                            </button>
                          {/if}
                          <button
                            class="icon-btn danger"
                            title="Remove source"
                            onclick={() => removeAlternate(app, k)}
                          >
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
                        onchange={(e) =>
                          (app.tags = e.currentTarget.value
                            .split(',')
                            .map((t) => t.trim())
                            .filter(Boolean))}
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
            {/if}
          {/each}

          <button class="btn btn-ghost add" onclick={() => addApp(cat, ci)}>
            <Plus size={15} /> Add app
          </button>
        </div>
      {/if}
    </section>
    {/if}
  {/each}

  <button class="btn add-cat" onclick={addCategory}>
    <Plus size={16} /> Add category
  </button>
{/if}

<style>
  .head {
    display: flex;
    align-items: center;
    gap: 14px;
    margin-bottom: 14px;
  }
  .head h1 {
    font-size: 1.4rem;
  }
  .spacer {
    flex: 1;
  }
  .status {
    font-size: 0.88rem;
    padding: 8px 12px;
    border-radius: var(--radius-sm);
    margin-bottom: 16px;
  }
  .status.ok {
    color: var(--success);
    background: color-mix(in srgb, var(--success) 12%, transparent);
  }
  .status.err {
    color: var(--danger);
    background: color-mix(in srgb, var(--danger) 12%, transparent);
  }
  .toolbar {
    display: flex;
    align-items: center;
    gap: 14px;
    margin-bottom: 14px;
  }
  .count {
    font-size: 0.82rem;
    flex-shrink: 0;
  }
  .search {
    max-width: 300px;
    margin-left: auto;
  }

  .cat {
    padding: 10px 12px;
    margin-bottom: 12px;
  }
  .cat-head {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .chip {
    font-family: var(--font-mono);
    font-size: 0.72rem;
    color: var(--text-muted);
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 1px 9px;
    flex-shrink: 0;
  }

  .apps {
    margin-top: 10px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .app {
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--surface-2);
  }
  .app.open {
    border-color: var(--border-strong);
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
  .cat-head .in {
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
    align-self: flex-start;
    margin-top: 4px;
    font-size: 0.85rem;
  }
  .add-cat {
    margin-top: 4px;
  }
</style>

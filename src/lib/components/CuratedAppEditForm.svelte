<script lang="ts">
  import { tick } from 'svelte';
  import { Plus, Trash2, X } from '@lucide/svelte';
  import * as api from '$lib/api';
  import { reloadCurated } from '$lib/stores/curated';
  import { confirmAction } from '$lib/stores/confirm';
  import type { CuratedApp, CuratedFile, Source, Variant } from '$lib/types';

  let {
    source,
    id,
    onClose,
    onSaved
  }: {
    source: Source;
    id: string;
    onClose: () => void;
    onSaved?: () => void;
  } = $props();

  const allSources: Source[] = ['winget', 'scoop', 'choco', 'msstore', 'local'];
  let idLower = $derived(id.trim().toLowerCase());

  let file = $state<CuratedFile | null>(null);
  let error = $state('');
  let saving = $state(false);
  let showMore = $state(false);
  let root = $state<HTMLDivElement | null>(null);

  // Editable copy of the app's fields.
  let name = $state('');
  let description = $state('');
  let homepage = $state('');
  let icon = $state('');
  let donate = $state('');
  let releaseNotes = $state('');
  let tags = $state('');
  let srcs = $state<Variant[]>([]);
  let categoryId = $state('');
  let isCustom = $state(true);
  let loaded = $state(false);

  $effect(() => {
    (async () => {
      try {
        const f = await api.getCurated();
        file = f;
        for (const cat of f.categories) {
          const app = cat.apps.find((a) => a.source === source && a.id.toLowerCase() === idLower);
          if (app) {
            name = app.name ?? '';
            description = app.description ?? '';
            homepage = app.homepage ?? '';
            icon = app.icon ?? '';
            donate = app.donate ?? '';
            releaseNotes = app.releaseNotes ?? '';
            tags = (app.tags ?? []).join(', ');
            srcs = [{ source: app.source, id: app.id }, ...app.alternates.map((a) => ({ ...a }))];
            categoryId = cat.id;
            isCustom = app.custom;
            loaded = true;
            await tick();
            root?.querySelector<HTMLInputElement>('input')?.focus();
            return;
          }
        }
        error = 'This app is no longer in the catalog.';
      } catch (e) {
        error = `Could not load the catalog: ${e}`;
      }
    })();
  });

  function addSource() {
    const used = new Set(srcs.map((s) => s.source));
    const next = allSources.find((s) => !used.has(s)) ?? 'local';
    srcs = [...srcs, { source: next, id: '' }];
  }
  function removeSource(i: number) {
    srcs = srcs.filter((_, k) => k !== i);
  }

  async function save() {
    if (!file || saving) return;
    const blank = (v: string) => (v.trim() ? v.trim() : null);

    // Clean + de-dupe sources; the first is the primary.
    const seen = new Set<Source>();
    const cleaned = srcs
      .map((v) => ({ source: v.source, id: v.id.trim() }))
      .filter((v) => (v.id || v.source === 'local') && !seen.has(v.source) && seen.add(v.source));
    if (cleaned.length === 0) {
      error = 'The app needs at least one source with a package id.';
      return;
    }
    const primary = cleaned[0];

    const updated: CuratedApp = {
      id: primary.id,
      source: primary.source,
      name: blank(name),
      description: blank(description),
      homepage: blank(homepage),
      icon: blank(icon),
      alternates: cleaned.slice(1),
      tags: tags
        .split(',')
        .map((t) => t.trim())
        .filter(Boolean),
      donate: blank(donate),
      releaseNotes: blank(releaseNotes),
      custom: isCustom
    };

    // Remove the existing entry (by original identity), then insert into the
    // chosen category. Built-ins keep their category (the picker is disabled),
    // so they land back where they were.
    let origCat = file.categories.find((c) =>
      c.apps.some((a) => a.source === source && a.id.toLowerCase() === idLower)
    );
    if (origCat) {
      origCat.apps = origCat.apps.filter(
        (a) => !(a.source === source && a.id.toLowerCase() === idLower)
      );
    }
    const target = file.categories.find((c) => c.id === categoryId) ?? origCat;
    if (!target) {
      error = 'Could not find a category to save into.';
      return;
    }
    target.apps.push(updated);

    saving = true;
    error = '';
    try {
      await api.saveCurated(file);
      await reloadCurated();
      onSaved?.();
      onClose();
    } catch (e) {
      error = `Save failed: ${e}`;
      saving = false;
    }
  }

  // Remove a user-added app from the list entirely (custom apps only).
  async function remove() {
    if (!file || saving) return;
    const ok = await confirmAction({
      title: 'Remove from your list?',
      message: `"${name || id}" will be removed from your Discover list.`,
      confirmLabel: 'Remove',
      danger: true
    });
    if (!ok) return;
    for (const cat of file.categories) {
      cat.apps = cat.apps.filter((a) => !(a.source === source && a.id.toLowerCase() === idLower));
    }
    saving = true;
    error = '';
    try {
      await api.saveCurated(file);
      await reloadCurated();
      onSaved?.();
      onClose();
    } catch (e) {
      error = `Remove failed: ${e}`;
      saving = false;
    }
  }
</script>

<div class="edit-pane" bind:this={root}>
  <div class="pane-head">
    <span class="pane-title">Edit app</span>
    <div class="spacer"></div>
    <button class="icon-btn" onclick={onClose} aria-label="Close edit" title="Close"><X size={16} /></button>
  </div>

  {#if !loaded && !error}
    <p class="muted pad">Loading…</p>
  {:else if error && !loaded}
    <p class="err pad">{error}</p>
  {:else}
    <div class="pane-scroll">
      <div class="form">
        {#if !isCustom}
          <p class="note">
            Built-in app - your changes are kept as a personal override on top of the catalog.
          </p>
        {/if}

        <label class="f">
          <span class="fl">Name</span>
          <input class="in" placeholder="optional" bind:value={name} />
        </label>
        <label class="f">
          <span class="fl">Description</span>
          <input class="in" placeholder="optional" bind:value={description} />
        </label>
        <label class="f">
          <span class="fl">Tags (comma-separated)</span>
          <input class="in" placeholder="open source, free, chromium" bind:value={tags} />
        </label>
        <label class="f">
          <span class="fl">Category</span>
          <select class="in" bind:value={categoryId} disabled={!isCustom} title={isCustom ? '' : 'Built-in apps stay in their category'}>
            {#each file?.categories ?? [] as c (c.id)}
              <option value={c.id}>{c.title || c.id}</option>
            {/each}
          </select>
        </label>

        {#if showMore}
          <div class="more">
            <div class="src-block">
              <span class="fl">Sources</span>
              {#each srcs as s, i (i)}
                <div class="src-row">
                  <select class="in src" bind:value={s.source}>
                    {#each allSources as src (src)}<option value={src}>{src}</option>{/each}
                  </select>
                  <input class="in mono" placeholder="package id" bind:value={s.id} />
                  <button class="icon-btn danger" title="Remove source" disabled={srcs.length === 1} onclick={() => removeSource(i)}>
                    <Trash2 size={14} />
                  </button>
                </div>
              {/each}
              <button class="btn btn-ghost add-src" onclick={addSource}>
                <Plus size={14} /> Add source
              </button>
            </div>
            <label class="f">
              <span class="fl">Homepage</span>
              <input class="in" placeholder="optional" bind:value={homepage} />
            </label>
            <label class="f">
              <span class="fl">Icon URL</span>
              <input class="in" placeholder="optional" bind:value={icon} />
            </label>
            <label class="f">
              <span class="fl">Donate URL</span>
              <input class="in" placeholder="optional" bind:value={donate} />
            </label>
            <label class="f">
              <span class="fl">Release notes URL</span>
              <input class="in" placeholder="optional" bind:value={releaseNotes} />
            </label>
          </div>
        {/if}

        <button class="btn btn-ghost more-toggle" onclick={() => (showMore = !showMore)}>
          {showMore ? 'Fewer options' : 'More options'}
        </button>

        {#if error}<p class="err">{error}</p>{/if}
      </div>
    </div>

    <div class="foot">
      {#if isCustom}
        <button class="btn btn-ghost danger" onclick={remove} disabled={saving}>
          <Trash2 size={14} /> Remove
        </button>
      {/if}
      <div class="spacer"></div>
      <button class="btn btn-ghost" onclick={onClose}>Cancel</button>
      <button class="btn btn-accent" onclick={save} disabled={saving}>
        {saving ? 'Saving…' : 'Save'}
      </button>
    </div>
  {/if}
</div>

<style>
  .edit-pane {
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
  .pane-title {
    font-size: 0.95rem;
    font-weight: 600;
  }
  .spacer {
    flex: 1;
  }
  .pane-scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
  }
  .form {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 18px 20px;
    max-width: 640px;
  }
  .note {
    font-size: 0.82rem;
    color: var(--text-muted);
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 8px 10px;
    margin: 0;
  }
  .f {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .fl {
    font-size: 0.72rem;
    color: var(--text-muted);
  }
  .more {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding-top: 12px;
    margin-top: 2px;
    border-top: 1px solid var(--border);
  }
  .more-toggle {
    align-self: flex-start;
    font-size: 0.82rem;
  }
  .src-block {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .src-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .src-row .src {
    flex: 0 0 110px;
  }
  .src-row .mono {
    flex: 1;
    font-family: var(--font-mono);
    font-size: 0.8rem;
  }
  .add-src {
    align-self: flex-start;
    font-size: 0.8rem;
    padding: 4px 8px;
  }
  .in {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    color: var(--text);
    padding: 7px 10px;
    font-size: 0.86rem;
    outline: none;
    width: 100%;
    min-width: 0;
  }
  .in:focus {
    border-color: var(--accent);
  }
  .in:disabled {
    opacity: 0.6;
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
  .err {
    font-size: 0.84rem;
    color: var(--danger);
    margin: 0;
  }
  .pad {
    padding: 20px;
  }
  .foot {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 20px;
    border-top: 1px solid var(--border);
  }
  .danger {
    color: var(--danger);
  }
  .danger:hover:not(:disabled) {
    background: color-mix(in srgb, var(--danger) 12%, transparent);
  }
</style>

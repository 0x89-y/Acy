<script lang="ts">
  import type { Snippet } from 'svelte';
  import { Download, Trash2 } from '@lucide/svelte';
  import * as api from '$lib/api';
  import { deleteIcon, loadIcon, redownloadIcon } from '$lib/stores/icons';
  import { settings } from '$lib/stores/settings';
  import type { Source } from '$lib/types';

  let {
    source,
    id,
    name = '',
    from = null,
    size = 48,
    children
  }: {
    source: Source;
    id: string;
    name?: string;
    from?: string | null;
    size?: number;
    children?: Snippet;
  } = $props();

  let iconUrl = $state<string | null>(null);
  let cached = $state(false);
  let deleted = $state(false);
  let busy = $state<'' | 'download' | 'delete'>('');
  let msg = $state('');

  let ready = $derived(id.trim().length > 0);
  let initial = $derived(((name || id).trim()[0] ?? '?').toUpperCase());

  let status = $derived.by(() => {
    if (!ready) return 'Add a package id first.';
    if (busy === 'download') return 'Downloading…';
    if (busy === 'delete') return 'Deleting…';
    if (msg) return msg;
    if (deleted) return "Deleted - Acy won't fetch it again on its own.";
    if (iconUrl) return 'Cached on this PC.';
    return 'Not downloaded yet.';
  });

  $effect(() => {
    const s = source;
    const i = id;
    iconUrl = null;
    cached = false;
    deleted = false;
    msg = '';
    if (!i.trim()) return;
    const t = setTimeout(() => void load(s, i), 250);
    return () => clearTimeout(t);
  });

  async function load(s: Source, i: string) {
    try {
      const st = await api.appIconState(s, i);
      if (source !== s || id !== i) return;
      cached = st.cached;
      deleted = st.deleted;
      iconUrl = st.cached ? await loadIcon(s, i, from, $settings.steamGridKey) : null;
    } catch {
    }
  }

  async function download() {
    if (busy || !ready) return;
    busy = 'download';
    msg = '';
    try {
      const url = await redownloadIcon(source, id, from);
      iconUrl = url;
      cached = !!url;
      deleted = false;
      if (!url) msg = 'No icon found - try setting an Icon URL or homepage.';
    } catch (e) {
      msg = `Download failed: ${e}`;
    } finally {
      busy = '';
    }
  }

  async function remove() {
    if (busy || !ready) return;
    busy = 'delete';
    msg = '';
    try {
      await deleteIcon(source, id);
      iconUrl = null;
      cached = false;
      deleted = true;
    } catch (e) {
      msg = `Delete failed: ${e}`;
    } finally {
      busy = '';
    }
  }
</script>

<div class="icon-wrap">
  {#if children}
    <div class="extra">{@render children()}</div>
  {/if}
  <div class="icon-row">
    <div class="preview" style="width:{size}px; height:{size}px;">
      {#if iconUrl}
        <img src={iconUrl} alt="" />
      {:else}
        <span class="ph" style="font-size:{Math.round(size * 0.34)}px;">{initial}</span>
      {/if}
    </div>
    <button
      class="btn btn-ghost"
      onclick={download}
      disabled={!!busy || !ready}
      title="Fetch from the icon URL, or the homepage"
    >
      <Download size={14} />
      {iconUrl || cached ? 'Re-download' : 'Download'}
    </button>
    <button
      class="icon-btn danger"
      onclick={remove}
      disabled={!!busy || !ready || (!iconUrl && !cached)}
      title="Delete this icon"
      aria-label="Delete this icon"
    >
      <Trash2 size={15} />
    </button>
    <span class="status" title={status}>{status}</span>
  </div>
</div>

<style>
  .icon-wrap {
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-width: 0;
  }
  .extra {
    display: flex;
    min-width: 0;
  }
  .icon-row {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
  }
  .preview {
    flex: 0 0 auto;
    display: grid;
    place-items: center;
    border-radius: 28%;
    background: var(--surface-2);
    border: 1px solid var(--border);
    overflow: hidden;
  }
  .preview img {
    width: 100%;
    height: 100%;
    object-fit: contain;
    padding: 5px;
  }
  .ph {
    font-family: var(--font-mono);
    font-weight: 600;
    color: var(--text-muted);
  }
  .btn {
    flex-shrink: 0;
    font-size: 0.82rem;
    padding: 5px 10px;
  }
  .icon-btn {
    display: inline-flex;
    flex-shrink: 0;
    padding: 6px;
    border: none;
    background: transparent;
    color: var(--text-muted);
    border-radius: var(--radius-sm);
    line-height: 0;
  }
  .icon-btn:hover:not(:disabled) {
    background: color-mix(in srgb, var(--danger) 12%, transparent);
    color: var(--danger);
  }
  .icon-btn:disabled {
    opacity: 0.35;
    cursor: default;
  }
  .status {
    font-size: 0.74rem;
    color: var(--text-muted);
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .danger {
    color: var(--danger);
  }
  .danger:hover:not(:disabled) {
    background: color-mix(in srgb, var(--danger) 12%, transparent);
  }
</style>

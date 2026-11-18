<script lang="ts">
  import { goto } from '$app/navigation';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { Check, Plus, Bookmark, BookmarkCheck } from '@lucide/svelte';
  import AppIcon from './AppIcon.svelte';
  import InstallButton from './InstallButton.svelte';
  import { settings } from '$lib/stores/settings';
  import { queuedKeys, toggleQueued } from '$lib/stores/queue';
  import { copyText } from '$lib/clipboard';
  import { runOp, installCommand } from '$lib/install';
  import { variantKey } from '$lib/catalog';
  import { openContextMenu, type CtxItem } from '$lib/stores/contextMenu';
  import type { Variant } from '$lib/types';

  let {
    name,
    description = null,
    variants,
    installed = false,
    homepage = null,
    iconCacheOnly = false,
    highlight = '',
    layout = 'grid',
    backTo = null,
    inList = false,
    ctxExtra = [],
    onChanged,
    onAddToList
  }: {
    name: string;
    description?: string | null;
    variants: Variant[];
    installed?: boolean;
    homepage?: string | null;
    iconCacheOnly?: boolean;
    layout?: 'grid' | 'list';
    highlight?: string;
    backTo?: string | null;
    /** Already in the user's catalog (search results from package managers). */
    inList?: boolean;
    ctxExtra?: CtxItem[];
    onChanged?: () => void;
    /** Shows an "add to my list" button (package-manager search results). */
    onAddToList?: () => void;
  } = $props();

  let primary = $derived(variants[0]);
  let href = $derived(
    `/app/${primary.source}/${encodeURIComponent(primary.id)}${backTo ? `?back=${encodeURIComponent(backTo)}` : ''}`
  );

  let chosen = $derived(
    ($settings.preferredSource && variants.find((v) => v.source === $settings.preferredSource)) ||
      variants[0]
  );

  let key = $derived(variantKey(primary));
  let queued = $derived($queuedKeys.has(key));
  const toggleQueue = () => toggleQueued({ key, name, variants });

  let nameParts = $derived.by(() => {
    const q = highlight.trim();
    if (!q) return [{ t: name, hit: false }];
    const lower = name.toLowerCase();
    const ql = q.toLowerCase();
    const out: { t: string; hit: boolean }[] = [];
    let i = 0;
    while (i < name.length) {
      const idx = lower.indexOf(ql, i);
      if (idx === -1) {
        out.push({ t: name.slice(i), hit: false });
        break;
      }
      if (idx > i) out.push({ t: name.slice(i, idx), hit: false });
      out.push({ t: name.slice(idx, idx + q.length), hit: true });
      i = idx + q.length;
    }
    return out;
  });

  async function doInstall() {
    await runOp('install', chosen.source, chosen.id, name);
    onChanged?.();
  }

  function onCtx(e: MouseEvent) {
    const items: CtxItem[] = [];
    if (!installed) {
      items.push({ label: 'Install', onSelect: doInstall });
      items.push({ label: queued ? 'Remove from queue' : 'Add to queue', onSelect: toggleQueue });
    }
    items.push({ label: 'Open details', onSelect: () => goto(href) });
    const cmd = installCommand(primary.source, primary.id);
    if (cmd) items.push({ label: 'Copy command', onSelect: () => copyText(cmd) });
    items.push({ label: 'Copy id', onSelect: () => copyText(primary.id) });
    if (homepage) items.push({ label: 'Open homepage', onSelect: () => openUrl(homepage) });
    openContextMenu(e, [...items, ...ctxExtra]);
  }
</script>

<div
  class="card app-card"
  class:list={layout === 'list'}
  class:queued
  oncontextmenu={onCtx}
  role="group"
  aria-label={name}
>
  <a class="main" {href}>
    <AppIcon
      {name}
      source={primary.source}
      id={primary.id}
      {homepage}
      cacheOnly={iconCacheOnly}
      size={layout === 'list' ? 34 : 44}
    />
    <div class="meta">
      <div class="name">
        {#each nameParts as p, i (i)}{#if p.hit}<mark>{p.t}</mark>{:else}{p.t}{/if}{/each}
      </div>
      {#if description}<div class="desc muted">{description}</div>{/if}
    </div>
  </a>
  <div class="foot">
    {#if onAddToList}
      {#if inList}
        <span class="icon-button in-list" title="In your list" aria-label="In your list">
          <BookmarkCheck size={16} />
        </span>
      {:else}
        <button class="icon-button" onclick={onAddToList} title="Add to my list" aria-label={`Add ${name} to my list`}>
          <Bookmark size={16} />
        </button>
      {/if}
    {/if}
    {#if installed}
      <span class="installed"><Check size={14} /> Installed</span>
    {:else}
      <button
        class="icon-button queue-btn"
        onclick={toggleQueue}
        aria-pressed={queued}
        aria-label={queued ? `Remove ${name} from the install queue` : `Add ${name} to the install queue`}
        title={queued ? 'In the install queue' : 'Add to install queue'}
      >
        {#if queued}<Check size={16} />{:else}<Plus size={16} />{/if}
      </button>
      <InstallButton source={chosen.source} id={chosen.id} {name} quiet onDone={onChanged} />
    {/if}
  </div>
</div>

<style>
  .app-card {
    position: relative;
    display: flex;
    flex-direction: column;
    padding: 14px;
    gap: 12px;
    transition: background 0.15s;
  }
  .app-card:not(.list) {
    border: none;
    border-top: 1px solid var(--border);
    border-left: 1px solid var(--border);
    border-radius: 0;
    background: transparent;
  }
  .app-card:hover {
    background: var(--surface-hover);
  }
  .app-card.queued {
    background: color-mix(in srgb, var(--accent) 9%, var(--surface));
  }
  .app-card.list {
    flex-direction: row;
    align-items: center;
    gap: 14px;
    padding: 10px 16px;
    border: none;
    border-radius: 0;
    border-top: 1px solid var(--border);
    background: transparent;
  }
  .app-card.list.queued {
    background: color-mix(in srgb, var(--accent) 9%, var(--surface));
  }
  .app-card.list:first-child {
    border-top: none;
  }
  .app-card.list .main {
    flex: 1;
    align-items: center;
  }
  .app-card.list .desc {
    -webkit-line-clamp: 1;
    line-clamp: 1;
    margin-top: 2px;
  }
  .app-card.list .foot {
    margin-top: 0;
    flex-shrink: 0;
  }
  .main {
    display: flex;
    gap: 12px;
    color: inherit;
    text-decoration: none;
    min-width: 0;
    align-items: flex-start;
  }
  .main:hover {
    text-decoration: none;
  }
  .meta {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .name {
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .name mark {
    background: transparent;
    color: var(--accent);
    font-weight: 700;
  }
  .desc {
    font-size: var(--fs-sm);
    margin-top: 4px;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .foot {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 6px;
    margin-top: auto;
    min-height: 37px;
  }
  .queue-btn[aria-pressed='true'] {
    background: var(--accent-fill);
    color: var(--accent-contrast);
  }
  .queue-btn[aria-pressed='true']:hover {
    background: var(--accent-hover);
    color: var(--accent-contrast);
  }
  .in-list {
    color: var(--success);
  }
  .installed {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: var(--fs-sm);
    color: var(--text-muted);
  }
  .installed :global(svg) {
    color: var(--success);
  }
</style>

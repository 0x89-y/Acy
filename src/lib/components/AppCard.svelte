<script lang="ts">
  import { goto } from '$app/navigation';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { Check, Plus } from '@lucide/svelte';
  import AppIcon from './AppIcon.svelte';
  import SourceBadge from './SourceBadge.svelte';
  import InstallButton from './InstallButton.svelte';
  import InstallSplitButton from './InstallSplitButton.svelte';
  import { settings } from '$lib/stores/settings';
  import { copyText } from '$lib/clipboard';
  import { runOp, installCommand } from '$lib/install';
  import { openContextMenu, type CtxItem } from '$lib/stores/contextMenu';
  import { filterByTag } from '$lib/stores/discover';
  import type { Source, Variant } from '$lib/types';
  import type { Snippet } from 'svelte';

  let {
    name,
    description = null,
    variants,
    installed = false,
    homepage = null,
    gameName = null,
    allowPick = false,
    selectable = false,
    selected = false,
    highlight = '',
    layout = 'grid',
    backTo = null,
    tags = [],
    inList = false,
    ctxExtra = [],
    action,
    menu = null,
    onToggleSelect,
    onChanged,
    onAddToList
  }: {
    name: string;
    description?: string | null;
    variants: Variant[];
    installed?: boolean;
    homepage?: string | null;
    /** Display name for a Games-bucket app; enables SteamGridDB-by-name icons. */
    gameName?: string | null;
    allowPick?: boolean;
    /** Free-form labels shown as small chips on the card. */
    tags?: string[];
    /** Card layout: a vertical tile (grid) or a horizontal row (list). */
    layout?: 'grid' | 'list';
    /** When true, the card is a selection toggle (multi-select install). */
    selectable?: boolean;
    selected?: boolean;
    /** Query to bold inside the name (search results). */
    highlight?: string;
    /** Optional in-app destination for the detail page's Back link. */
    backTo?: string | null;
    /** True when this app is already in the user's curated list. */
    inList?: boolean;
    /** Extra right-click menu items appended by the parent (e.g. move-to-category). */
    ctxExtra?: CtxItem[];
    /** Custom button-area content (e.g. Uninstall / Update). Replaces the
     *  default Install button when set. Receives the primary source/id/name. */
    action?: Snippet<[{ source: Source; id: string; name: string }]>;
    /** Full right-click menu; when set, replaces the built-in install menu
     *  (ctxExtra is still appended). */
    menu?: CtxItem[] | null;
    onToggleSelect?: () => void;
    onChanged?: () => void;
    /** When set, shows a "+ Add" button that adds this app to the user's list. */
    onAddToList?: () => void;
  } = $props();

  let primary = $derived(variants[0]);
  let href = $derived(
    `/app/${primary.source}/${encodeURIComponent(primary.id)}${backTo ? `?back=${encodeURIComponent(backTo)}` : ''}`
  );

  // The source a one-click install uses: preferred if offered, else the first.
  let chosen = $derived(
    ($settings.preferredSource && variants.find((v) => v.source === $settings.preferredSource)) ||
      variants[0]
  );

  // Split the name into highlighted / plain segments for search matches.
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
    if (selectable) return;
    if (menu) {
      openContextMenu(e, [...menu, ...ctxExtra]);
      return;
    }
    const items: CtxItem[] = [];
    if (!installed) items.push({ label: 'Install', onSelect: doInstall });
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
  class:selected={selectable && selected}
  class:has-add={!!onAddToList && !selectable}
  oncontextmenu={onCtx}
  role="group"
>
  <a
    class="main"
    {href}
    onclick={(e) => {
      if (selectable) {
        e.preventDefault();
        onToggleSelect?.();
      }
    }}
  >
    <AppIcon
      {name}
      source={primary.source}
      id={primary.id}
      {homepage}
      {gameName}
      size={layout === 'list' ? 34 : 44}
    />
    <div class="meta">
      <div class="name">
        {#each nameParts as p, i (i)}{#if p.hit}<mark>{p.t}</mark>{:else}{p.t}{/if}{/each}
      </div>
      {#if description}<div class="desc muted">{description}</div>{/if}
      {#if tags.length}
        <div class="tags">
          {#each tags.slice(0, 3) as t (t)}
            <span
              class="tag"
              role="button"
              tabindex="0"
              title={`Filter Discover by "${t}"`}
              onclick={(e) => {
                e.preventDefault();
                e.stopPropagation();
                filterByTag(t);
              }}
              onkeydown={(e) => {
                if (e.key === 'Enter' || e.key === ' ') {
                  e.preventDefault();
                  e.stopPropagation();
                  filterByTag(t);
                }
              }}
            >
              {t}
            </span>
          {/each}
        </div>
      {/if}
    </div>
  </a>
  <div class="foot">
    <div class="badges">
      {#each variants as v (v.source)}<SourceBadge source={v.source} />{/each}
    </div>
    <div class="foot-right">
      {#if onAddToList && !selectable}
        {#if inList}
          <span class="add-corner is-in" title="In your list" aria-label="In your list">
            <Check size={14} />
          </span>
        {:else}
          <button
            class="add-corner"
            onclick={onAddToList}
            title="Add to my list"
            aria-label="Add to my list"
          >
            <Plus size={16} />
          </button>
        {/if}
      {/if}
    <div class="card-action">
      {#if selectable}
        <label class="acheck selection-action">
          <input
            type="checkbox"
            checked={selected}
            tabindex="-1"
            aria-label={`Select ${name}`}
            onchange={onToggleSelect}
          />
          <span class="box"><Check size={13} /></span>
          <span>{selected ? 'Selected' : 'Select'}</span>
        </label>
      {:else if action}
        {@render action({ source: primary.source, id: primary.id, name })}
      {:else if installed}
        <span class="installed">Installed</span>
      {:else if allowPick && variants.length > 1}
        <InstallSplitButton {variants} {name} preferred={$settings.preferredSource} onDone={onChanged} />
      {:else}
        <InstallButton source={primary.source} id={primary.id} {name} onDone={onChanged} />
      {/if}
    </div>
    </div>
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
  /* Grid tiles form a divided grid: flush cells separated by hairlines (not
     cards-in-a-card). Each cell draws its top + left hairline; the grid pulls
     the outermost ones under the panel frame (see +page.svelte). */
  .app-card:not(.list) {
    border: none;
    border-top: 1px solid var(--border);
    border-left: 1px solid var(--border);
    border-radius: 0;
    background: transparent;
  }
  .app-card:not(.list):hover {
    background: var(--surface-hover);
  }
  .app-card:not(.list).selected {
    background: color-mix(in srgb, var(--accent) 16%, var(--surface));
  }
  /* List layout is a divided list: flush rows inside one bordered container
     (.list-flow), separated by hairlines - no per-card border or radius. */
  .app-card.list {
    flex-direction: row;
    align-items: center;
    gap: 14px;
    padding: 10px 14px;
    border: none;
    border-radius: 0;
    border-top: 1px solid var(--border);
    background: transparent;
  }
  .app-card.list:first-child {
    border-top: none;
  }
  .app-card.list:hover {
    background: var(--surface-hover);
  }
  .app-card.list.selected {
    background: color-mix(in srgb, var(--accent) 16%, var(--surface));
  }
  .app-card.list .main {
    flex: 1;
    align-items: center;
  }
  .app-card.list .desc {
    display: -webkit-box;
    -webkit-line-clamp: 1;
    line-clamp: 1;
    margin-top: 2px;
  }
  .app-card.list .foot {
    margin-top: 0;
    flex-shrink: 0;
    gap: 12px;
  }
  .main {
    display: flex;
    gap: 12px;
    color: inherit;
    text-decoration: none;
    min-width: 0;
    align-items: flex-start;
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
    font-size: 0.82rem;
    margin-top: 4px;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .tags {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin-top: 5px;
  }
  .tags .tag {
    font-size: 0.66rem;
    font-family: var(--font-mono);
    color: var(--text-muted);
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 1px 7px;
    white-space: nowrap;
    cursor: pointer;
  }
  .tags .tag:hover {
    color: var(--accent);
    border-color: var(--accent);
  }
  .app-card.list .tags {
    flex-wrap: nowrap;
    overflow: hidden;
  }
  .foot {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    margin-top: auto;
  }
  .badges {
    display: flex;
    gap: 5px;
    flex-wrap: wrap;
  }
  .foot-right {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
  }
  .add-corner {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 26px;
    padding: 0;
    border-radius: var(--radius-sm);
    flex-shrink: 0;
  }
  button.add-corner {
    border: 1px solid var(--border-strong);
    background: var(--surface);
    color: var(--text-muted);
    transition: background 0.15s, color 0.15s, border-color 0.15s;
  }
  button.add-corner:hover {
    background: var(--surface-hover);
    color: var(--text);
    border-color: var(--accent);
  }
  .add-corner.is-in {
    color: var(--success);
  }
  /* Grid tiles: pin the "+" to the top-right corner, out of the footer flow. */
  .app-card:not(.list) .add-corner {
    position: absolute;
    top: 12px;
    right: 12px;
  }
  .app-card:not(.list).has-add .main {
    padding-right: 32px;
  }
  .card-action {
    width: 104px;
    height: 39px;
    display: flex;
    align-items: center;
    justify-content: flex-end;
    flex-shrink: 0;
  }
  .installed {
    font-size: 0.8rem;
    color: var(--success);
    font-weight: 500;
  }
  .selection-action {
    justify-content: flex-end;
    gap: 7px;
    color: var(--text-muted);
    font-size: 0.8rem;
    font-weight: 500;
  }
</style>

<script lang="ts">
  import { goto } from '$app/navigation';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import AppIcon from './AppIcon.svelte';
  import SourceBadge from './SourceBadge.svelte';
  import InstallButton from './InstallButton.svelte';
  import InstallSplitButton from './InstallSplitButton.svelte';
  import { settings } from '$lib/stores/settings';
  import { copyText } from '$lib/clipboard';
  import { runOp } from '$lib/install';
  import { openContextMenu, type CtxItem } from '$lib/stores/contextMenu';
  import type { Variant } from '$lib/types';

  let {
    name,
    description = null,
    variants,
    installed = false,
    sub = null,
    homepage = null,
    allowPick = false,
    selectable = false,
    selected = false,
    highlight = '',
    onToggleSelect,
    onChanged
  }: {
    name: string;
    description?: string | null;
    variants: Variant[];
    installed?: boolean;
    sub?: string | null;
    homepage?: string | null;
    allowPick?: boolean;
    selectable?: boolean;
    selected?: boolean;
    highlight?: string;
    onToggleSelect?: () => void;
    onChanged?: () => void;
  } = $props();

  let primary = $derived(variants[0]);
  let href = $derived(`/app/${primary.source}/${encodeURIComponent(primary.id)}`);

  let chosen = $derived(
    ($settings.preferredSource && variants.find((v) => v.source === $settings.preferredSource)) ||
      variants[0]
  );

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
    const items: CtxItem[] = [];
    if (!installed) items.push({ label: 'Install', onSelect: doInstall });
    items.push({ label: 'Open details', onSelect: () => goto(href) });
    items.push({ label: 'Copy id', onSelect: () => copyText(primary.id) });
    if (homepage) items.push({ label: 'Open homepage', onSelect: () => openUrl(homepage) });
    openContextMenu(e, items);
  }
</script>

<div class="card app-card" class:selected={selectable && selected} oncontextmenu={onCtx} role="group">
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
    {#if selectable}
      <span class="sel-check"><input type="checkbox" checked={selected} tabindex="-1" /></span>
    {/if}
    <AppIcon {name} source={primary.source} id={primary.id} {homepage} />
    <div class="meta">
      <div class="name">
        {#each nameParts as p, i (i)}{#if p.hit}<mark>{p.t}</mark>{:else}{p.t}{/if}{/each}
      </div>
      <div class="sub mono">{sub ?? primary.id}</div>
      {#if description}<div class="desc muted">{description}</div>{/if}
    </div>
  </a>
  <div class="foot">
    <div class="badges">
      {#each variants as v (v.source)}<SourceBadge source={v.source} />{/each}
    </div>
    {#if selectable}
      <span class="installed" class:dim={!selected}>{selected ? 'Selected' : ''}</span>
    {:else if installed}
      <span class="installed">Installed</span>
    {:else if allowPick && variants.length > 1}
      <InstallSplitButton {variants} {name} preferred={$settings.preferredSource} onDone={onChanged} />
    {:else}
      <InstallButton source={primary.source} id={primary.id} {name} onDone={onChanged} />
    {/if}
  </div>
</div>

<style>
  .app-card {
    display: flex;
    flex-direction: column;
    padding: 14px;
    gap: 12px;
    transition: border-color 0.15s, box-shadow 0.15s;
  }
  .app-card:hover {
    border-color: var(--border-strong);
    box-shadow: var(--shadow);
  }
  .app-card.selected {
    border-color: var(--accent);
  }
  .main {
    display: flex;
    gap: 12px;
    color: inherit;
    text-decoration: none;
    min-width: 0;
    align-items: flex-start;
  }
  .sel-check {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    padding-top: 2px;
  }
  .sel-check input {
    width: 17px;
    height: 17px;
    accent-color: var(--accent);
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
  .sub {
    font-size: 0.72rem;
    color: var(--text-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
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
  .installed {
    font-size: 0.8rem;
    color: var(--success);
    font-weight: 500;
  }
  .installed.dim {
    color: transparent;
  }
</style>

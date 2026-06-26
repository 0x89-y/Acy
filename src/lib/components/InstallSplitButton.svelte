<script lang="ts">
  import { ChevronDown } from '@lucide/svelte';
  import { runOp } from '$lib/install';
  import type { Source, Variant } from '$lib/types';

  let {
    variants,
    name,
    preferred = null,
    onDone
  }: {
    variants: Variant[];
    name: string;
    preferred?: Source | null;
    onDone?: () => void;
  } = $props();

  const labels: Record<Source, string> = {
    winget: 'winget',
    scoop: 'Scoop',
    choco: 'Chocolatey',
    msstore: 'Microsoft Store',
    local: 'a local file'
  };
  const menuLabel = (s: Source) => (s === 'local' ? 'Install from file…' : `Install with ${labels[s]}`);

  let busy = $state(false);
  let open = $state(false);
  let root = $state<HTMLElement | null>(null);

  // The source the main button uses: the preferred one if this app offers it,
  // otherwise the first listed variant.
  let chosen = $derived(
    (preferred && variants.find((v) => v.source === preferred)) || variants[0]
  );

  async function install(v: Variant) {
    open = false;
    busy = true;
    await runOp('install', v.source, v.id, name);
    busy = false;
    onDone?.();
  }

  function onWindowClick(e: MouseEvent) {
    if (open && root && !root.contains(e.target as Node)) open = false;
  }
</script>

<svelte:window onclick={onWindowClick} />

<div class="split" bind:this={root}>
  <button class="btn btn-accent main" onclick={() => install(chosen)} disabled={busy}>
    {busy ? 'Working…' : 'Install'}
  </button>
  <button
    class="btn btn-accent caret"
    onclick={() => (open = !open)}
    disabled={busy}
    aria-label="Choose source"
    aria-expanded={open}
  >
    <ChevronDown size={14} />
  </button>

  {#if open}
    <div class="menu card">
      {#each variants as v (v.source)}
        <button class="menu-item" onclick={() => install(v)}>
          <span>{menuLabel(v.source)}</span>
          {#if v.source === chosen.source}<span class="def mono">default</span>{/if}
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .split {
    position: relative;
    display: inline-flex;
    align-items: stretch;
  }
  .main {
    border-top-right-radius: 0;
    border-bottom-right-radius: 0;
  }
  .caret {
    border-top-left-radius: 0;
    border-bottom-left-radius: 0;
    border-left: 1px solid color-mix(in srgb, var(--accent-contrast) 35%, transparent);
    padding: 8px 8px;
  }
  .menu {
    position: absolute;
    right: 0;
    top: calc(100% + 6px);
    z-index: 30;
    min-width: 190px;
    padding: 4px;
    box-shadow: var(--shadow);
    display: flex;
    flex-direction: column;
  }
  .menu-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    width: 100%;
    border: none;
    background: transparent;
    color: var(--text);
    text-align: left;
    padding: 8px 10px;
    border-radius: var(--radius-sm);
    font-size: 0.86rem;
  }
  .menu-item:hover {
    background: var(--surface-hover);
  }
  .def {
    font-size: 0.66rem;
    color: var(--text-muted);
  }
</style>

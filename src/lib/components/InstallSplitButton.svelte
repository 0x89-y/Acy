<script lang="ts">
  import { tick } from 'svelte';
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
    msstore: 'Microsoft Store'
  };
  const menuLabel = (s: Source) => `Install with ${labels[s]}`;

  let busy = $state(false);
  let open = $state(false);
  let root = $state<HTMLElement | null>(null);
  let caret = $state<HTMLButtonElement | null>(null);
  let menu = $state<HTMLDivElement | null>(null);

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

  async function toggle() {
    open = !open;
    if (open) {
      await tick();
      menu?.querySelector<HTMLButtonElement>('button')?.focus();
    }
  }

  function onKeydown(e: KeyboardEvent) {
    if (!open || !menu) return;
    if (e.key === 'Escape') {
      e.preventDefault();
      open = false;
      caret?.focus();
      return;
    }
    if (e.key === 'Tab') {
      open = false;
      return;
    }
    if (e.key !== 'ArrowDown' && e.key !== 'ArrowUp') return;
    e.preventDefault();
    const items = Array.from(menu.querySelectorAll<HTMLButtonElement>('button'));
    const i = items.indexOf(document.activeElement as HTMLButtonElement);
    const step = e.key === 'ArrowDown' ? 1 : -1;
    items[(i + step + items.length) % items.length]?.focus();
  }
</script>

<svelte:window onclick={onWindowClick} onkeydown={onKeydown} />

<div class="split" bind:this={root}>
  <button class="btn btn-accent main" onclick={() => install(chosen)} disabled={busy}>
    {busy ? 'Working…' : 'Install'}
  </button>
  <button
    class="btn btn-accent caret"
    bind:this={caret}
    onclick={toggle}
    disabled={busy}
    aria-label="Choose source"
    aria-haspopup="menu"
    aria-expanded={open}
  >
    <ChevronDown size={14} />
  </button>

  {#if open}
    <div class="menu card" role="menu" aria-label="Install with" bind:this={menu}>
      {#each variants as v (v.source)}
        <button class="menu-item" role="menuitem" onclick={() => install(v)}>
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
    background: var(--accent-fill);
    border-radius: var(--radius);
  }
  .split .main,
  .split .caret {
    background: transparent;
    border-color: transparent;
  }
  .split .caret {
    border-left: 1px solid color-mix(in srgb, var(--accent-contrast) 35%, transparent);
  }
  .split .main:hover:not(:disabled),
  .split .caret:hover:not(:disabled) {
    background: rgba(0, 0, 0, 0.14);
  }
  .main {
    flex: 1;
    border-top-right-radius: 0;
    border-bottom-right-radius: 0;
  }
  .caret {
    border-top-left-radius: 0;
    border-bottom-left-radius: 0;
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
    font-size: var(--fs-md);
  }
  .menu-item:hover {
    background: var(--surface-hover);
  }
  .def {
    font-size: var(--fs-2xs);
    color: var(--text-muted);
  }
</style>

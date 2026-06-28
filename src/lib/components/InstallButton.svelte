<script lang="ts">
  import { onDestroy } from 'svelte';
  import { runOp, VERBS, type InstallKind } from '$lib/install';
  import type { Source } from '$lib/types';

  let {
    source,
    id,
    name = id,
    kind = 'install',
    onDone
  }: {
    source: Source;
    id: string;
    name?: string;
    kind?: InstallKind;
    onDone?: () => void;
  } = $props();

  let busy = $state(false);
  // Uninstall is a two-step confirm: first click arms it, second click runs.
  let confirming = $state(false);
  let confirmTimer: ReturnType<typeof setTimeout> | undefined;
  onDestroy(() => clearTimeout(confirmTimer));

  async function run() {
    busy = true;
    await runOp(kind, source, id, name);
    busy = false;
    onDone?.();
  }

  function onClick() {
    if (kind === 'uninstall' && !confirming) {
      confirming = true;
      clearTimeout(confirmTimer);
      confirmTimer = setTimeout(() => (confirming = false), 3000);
      return;
    }
    clearTimeout(confirmTimer);
    confirming = false;
    run();
  }

  let cls = $derived(kind === 'uninstall' ? 'btn' : 'btn btn-accent');
</script>

<button class={cls} class:confirm={confirming} onclick={onClick} disabled={busy}>
  {busy ? 'Working…' : confirming ? 'Remove?' : VERBS[kind]}
</button>

<style>
  .confirm {
    background: var(--danger);
    border-color: var(--danger);
    color: #fff;
  }
  .confirm:hover:not(:disabled) {
    background: var(--danger);
    filter: brightness(1.08);
  }
</style>

<script lang="ts">
  import { runOp, VERBS, type InstallKind } from '$lib/install';
  import type { Source } from '$lib/types';
  import ConfirmAction from './ConfirmAction.svelte';

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

  async function run() {
    busy = true;
    await runOp(kind, source, id, name);
    busy = false;
    onDone?.();
  }

  let cls = $derived(kind === 'uninstall' ? 'btn' : 'btn btn-accent');
</script>

{#if kind === 'uninstall'}
  <ConfirmAction
    label={VERBS[kind]}
    message={`Remove ${name}?`}
    confirmLabel="Uninstall"
    {busy}
    onConfirm={run}
  />
{:else}
  <button class={cls} onclick={run} disabled={busy}>
    {busy ? 'Working…' : VERBS[kind]}
  </button>
{/if}

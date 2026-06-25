<script lang="ts">
  import { enqueue } from '$lib/stores/ops';
  import * as api from '$lib/api';
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
    kind?: 'install' | 'update' | 'uninstall';
    onDone?: () => void;
  } = $props();

  let busy = $state(false);
  const verbs = { install: 'Install', update: 'Update', uninstall: 'Uninstall' };

  // Trust the resulting state over the exit code: some uninstallers report a
  // non-zero code even on success.
  async function verify(): Promise<boolean> {
    const idLower = id.toLowerCase();
    if (kind === 'update') {
      const ups = await api.listUpdates([source]);
      return !ups.some((p) => p.id.toLowerCase() === idLower);
    }
    const inst = await api.listInstalled([source]);
    const present = inst.some((p) => p.id.toLowerCase() === idLower);
    return kind === 'uninstall' ? !present : present;
  }

  async function run() {
    busy = true;
    await enqueue(
      `${verbs[kind]} ${name}`,
      (opId) => {
        if (kind === 'install') return api.install(source, id, opId);
        if (kind === 'update') return api.upgrade(source, id, opId);
        return api.uninstall(source, id, opId);
      },
      verify
    );
    busy = false;
    onDone?.();
  }

  let cls = $derived(kind === 'uninstall' ? 'btn' : 'btn btn-accent');
</script>

<button class={cls} onclick={run} disabled={busy}>
  {busy ? 'Working…' : verbs[kind]}
</button>

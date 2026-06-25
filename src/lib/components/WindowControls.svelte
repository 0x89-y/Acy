<script lang="ts">
  import { onMount } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { Minus, Square, Copy, X } from '@lucide/svelte';

  let maximized = $state(false);

  const win = () => getCurrentWindow();

  onMount(() => {
    let unlisten: (() => void) | undefined;
    (async () => {
      try {
        const w = win();
        maximized = await w.isMaximized();
        unlisten = await w.onResized(async () => {
          maximized = await w.isMaximized();
        });
      } catch (e) {
        console.error('window controls init failed', e);
      }
    })();
    return () => unlisten?.();
  });
</script>

<div class="controls">
  <button class="ctl" onclick={() => win().minimize()} aria-label="Minimize" title="Minimize">
    <Minus size={15} />
  </button>
  <button
    class="ctl"
    onclick={() => win().toggleMaximize()}
    aria-label="Maximize"
    title={maximized ? 'Restore' : 'Maximize'}
  >
    {#if maximized}<Copy size={13} />{:else}<Square size={13} />{/if}
  </button>
  <button class="ctl close" onclick={() => win().close()} aria-label="Close" title="Close">
    <X size={16} />
  </button>
</div>

<style>
  .controls {
    display: flex;
    align-items: center;
    gap: 2px;
  }
  .ctl {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 34px;
    height: 30px;
    border: none;
    background: transparent;
    color: var(--text-muted);
    border-radius: var(--radius-sm);
    line-height: 0;
  }
  .ctl:hover {
    background: var(--surface-hover);
    color: var(--text);
  }
  .ctl.close:hover {
    background: var(--danger);
    color: #fff;
  }
</style>

<script lang="ts">
  import { X } from '@lucide/svelte';
  import { updaterPhase, updaterVersion, installUpdate } from '$lib/stores/updater';

  let dismissed = $state(false);
  let lastVersion = $state<string | null>(null);

  $effect(() => {
    if ($updaterVersion && $updaterVersion !== lastVersion) {
      dismissed = false;
      lastVersion = $updaterVersion;
    }
  });

  let show = $derived(
    ($updaterPhase === 'available' || $updaterPhase === 'downloading') && !dismissed
  );
</script>

{#if show}
  <div class="app-update card">
    {#if $updaterPhase === 'downloading'}
      <span class="txt">Downloading Acy update…</span>
    {:else}
      <span class="txt">Acy <strong>v{$updaterVersion}</strong> is available.</span>
      <button class="btn btn-accent sm" onclick={installUpdate}>Update</button>
      <button class="x" onclick={() => (dismissed = true)} aria-label="Dismiss">
        <X size={15} />
      </button>
    {/if}
  </div>
{/if}

<style>
  .app-update {
    position: fixed;
    left: 18px;
    bottom: 18px;
    z-index: 45;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 10px 10px 16px;
    box-shadow: var(--shadow);
    max-width: min(380px, calc(100vw - 36px));
  }
  .txt {
    flex: 1;
    min-width: 0;
    font-size: 0.88rem;
    line-height: 1.4;
  }
  .sm {
    flex-shrink: 0;
    padding: 6px 14px;
    font-size: 0.84rem;
  }
  .x {
    display: inline-flex;
    flex-shrink: 0;
    padding: 4px;
    border: none;
    background: transparent;
    color: var(--text-muted);
    border-radius: var(--radius-sm);
    line-height: 0;
  }
  .x:hover {
    background: var(--surface-hover);
    color: var(--text);
  }
</style>

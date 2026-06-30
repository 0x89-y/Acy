<script lang="ts">
  import { tick } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { closePromptOpen } from '$lib/stores/tray';
  import { setCloseToTray, setAskCloseToTray } from '$lib/stores/settings';

  let dontAsk = $state(false);
  let dialog = $state<HTMLDivElement | null>(null);
  let previousFocus: HTMLElement | null = null;

  function applyDontAsk() {
    if (dontAsk) setAskCloseToTray(false);
  }

  // Keep running in the tray, and remember the choice so it becomes the default.
  function minimize() {
    setCloseToTray(true);
    applyDontAsk();
    closePromptOpen.set(false);
    getCurrentWindow().hide();
  }

  function quit() {
    applyDontAsk();
    closePromptOpen.set(false);
    getCurrentWindow().destroy();
  }

  function cancel() {
    closePromptOpen.set(false);
  }

  $effect(() => {
    if (!$closePromptOpen || typeof document === 'undefined') return;
    previousFocus = document.activeElement as HTMLElement | null;
    void tick().then(() => dialog?.querySelector<HTMLElement>('[data-cancel]')?.focus());
    return () => previousFocus?.focus();
  });

  function onKeydown(e: KeyboardEvent) {
    if (!$closePromptOpen) return;
    if (e.key === 'Escape') {
      e.preventDefault();
      cancel();
      return;
    }
    if (e.key !== 'Tab' || !dialog) return;
    const items = Array.from(dialog.querySelectorAll<HTMLElement>('button, input'));
    const first = items[0];
    const last = items.at(-1);
    if (e.shiftKey && document.activeElement === first) {
      e.preventDefault();
      last?.focus();
    } else if (!e.shiftKey && document.activeElement === last) {
      e.preventDefault();
      first?.focus();
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

{#if $closePromptOpen}
  <div class="backdrop">
    <button class="backdrop-close" onclick={cancel} aria-label="Cancel closing Acy"></button>
    <div class="dialog card" role="dialog" aria-modal="true" aria-labelledby="tray-prompt-title" bind:this={dialog}>
      <h2 id="tray-prompt-title">Keep Acy running in the tray?</h2>
      <p class="muted">
        Acy can stay in the system tray and check for updates in the background instead of fully
        closing. You can change this any time in Settings.
      </p>
      <label class="dontask">
        <input type="checkbox" bind:checked={dontAsk} />
        Don't ask again
      </label>
      <div class="actions">
        <button class="btn btn-ghost" data-cancel onclick={cancel}>Cancel</button>
        <button class="btn" onclick={quit}>Quit Acy</button>
        <button class="btn btn-accent" onclick={minimize}>Minimize to tray</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 80;
    background: rgba(0, 0, 0, 0.45);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
  }
  .backdrop-close {
    position: absolute;
    inset: 0;
    border: 0;
    background: transparent;
    cursor: default;
  }
  .dialog {
    position: relative;
    z-index: 1;
    width: min(420px, 100%);
    padding: 22px;
    box-shadow: var(--shadow);
  }
  .dialog h2 {
    font-size: 1.1rem;
    margin-bottom: 10px;
  }
  .dialog p {
    font-size: 0.9rem;
    line-height: 1.5;
    margin: 0 0 16px;
  }
  .dontask {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    font-size: 0.86rem;
    color: var(--text-muted);
    cursor: pointer;
    margin-bottom: 18px;
  }
  .dontask input {
    width: 15px;
    height: 15px;
    accent-color: var(--accent);
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
  }
</style>

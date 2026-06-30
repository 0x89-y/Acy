<script lang="ts">
  import { tick } from 'svelte';

  let {
    label,
    message,
    confirmLabel = 'Confirm',
    busyLabel = 'Working…',
    busy = false,
    className = 'btn',
    onConfirm
  }: {
    label: string;
    message: string;
    confirmLabel?: string;
    busyLabel?: string;
    busy?: boolean;
    className?: string;
    onConfirm: () => void | Promise<void>;
  } = $props();

  let open = $state(false);
  let root = $state<HTMLDivElement | null>(null);
  let cancelButton = $state<HTMLButtonElement | null>(null);
  let trigger = $state<HTMLButtonElement | null>(null);
  let popover = $state<HTMLDivElement | null>(null);

  $effect(() => {
    if (!open) return;
    void tick().then(() => cancelButton?.focus());
  });

  function close(restoreFocus = true) {
    open = false;
    if (restoreFocus) void tick().then(() => trigger?.focus());
  }

  function onWindowClick(e: MouseEvent) {
    if (open && !root?.contains(e.target as Node)) close(false);
  }

  function onKeydown(e: KeyboardEvent) {
    if (!open) return;
    if (e.key === 'Escape') {
      e.preventDefault();
      close();
      return;
    }
    if (e.key !== 'Tab' || !popover) return;
    const focusable = Array.from(popover.querySelectorAll<HTMLButtonElement>('button:not(:disabled)'));
    const first = focusable[0];
    const last = focusable.at(-1);
    if (e.shiftKey && document.activeElement === first) {
      e.preventDefault();
      last?.focus();
    } else if (!e.shiftKey && document.activeElement === last) {
      e.preventDefault();
      first?.focus();
    }
  }

  async function confirm() {
    open = false;
    await onConfirm();
  }
</script>

<svelte:window onclick={onWindowClick} onkeydown={onKeydown} />

<div class="confirm-wrap" bind:this={root}>
  <button
    type="button"
    class={className}
    bind:this={trigger}
    onclick={() => (open = !open)}
    disabled={busy}
    aria-haspopup="dialog"
    aria-expanded={open}
  >
    {busy ? busyLabel : label}
  </button>
  {#if open}
    <div class="confirm-pop card" role="dialog" aria-label={message} bind:this={popover}>
      <p>{message}</p>
      <div class="actions">
        <button type="button" class="btn btn-ghost compact" bind:this={cancelButton} onclick={() => close()}>
          Cancel
        </button>
        <button type="button" class="btn danger compact" onclick={confirm}>{confirmLabel}</button>
      </div>
    </div>
  {/if}
</div>

<style>
  .confirm-wrap {
    position: relative;
    display: inline-flex;
  }
  .confirm-pop {
    position: absolute;
    z-index: 35;
    right: 0;
    bottom: calc(100% + 7px);
    width: max-content;
    max-width: min(280px, calc(100vw - 36px));
    padding: 12px;
    box-shadow: var(--shadow);
  }
  .confirm-pop p {
    margin: 0 0 10px;
    font-size: 0.86rem;
    color: var(--text);
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 7px;
  }
  .compact {
    padding: 5px 10px;
    font-size: 0.82rem;
  }
  .danger {
    color: #fff;
    background: var(--danger);
    border-color: var(--danger);
  }
  .danger:hover:not(:disabled) {
    background: var(--danger);
    filter: brightness(1.08);
  }
</style>

<script lang="ts">
  import { tick } from 'svelte';
  import { confirmRequest } from '$lib/stores/confirm';

  let dialog = $state<HTMLDivElement | null>(null);
  let previousFocus: HTMLElement | null = null;
  let remember = $state(false);

  function respond(ok: boolean) {
    const req = $confirmRequest;
    confirmRequest.set(null);
    req?.resolve({ ok, remember });
  }

  $effect(() => {
    if (!$confirmRequest || typeof document === 'undefined') return;
    remember = false;
    previousFocus = document.activeElement as HTMLElement | null;
    void tick().then(() => dialog?.querySelector<HTMLElement>('[data-confirm]')?.focus());
    return () => previousFocus?.focus();
  });

  function onKeydown(e: KeyboardEvent) {
    if (!$confirmRequest) return;
    if (e.key === 'Escape') {
      e.preventDefault();
      respond(false);
      return;
    }
    if (e.key !== 'Tab' || !dialog) return;
    const items = Array.from(dialog.querySelectorAll<HTMLElement>('button'));
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

{#if $confirmRequest}
  <div class="backdrop">
    <button class="backdrop-close" onclick={() => respond(false)} aria-label="Cancel"></button>
    <div
      class="dialog card"
      role={$confirmRequest.alert ? 'alertdialog' : 'dialog'}
      aria-modal="true"
      aria-labelledby="confirm-title"
      aria-describedby={$confirmRequest.message ? 'confirm-message' : undefined}
      bind:this={dialog}
    >
      <h2 id="confirm-title">{$confirmRequest.title}</h2>
      {#if $confirmRequest.message}<p class="muted" id="confirm-message">{$confirmRequest.message}</p>{/if}
      {#if $confirmRequest.rememberLabel}
        <label class="remember">
          <input class="check-box" type="checkbox" bind:checked={remember} />
          {$confirmRequest.rememberLabel}
        </label>
      {/if}
      <div class="actions">
        {#if !$confirmRequest.alert}
          <button class="btn btn-ghost" onclick={() => respond(false)}>
            {$confirmRequest.cancelLabel ?? 'Cancel'}
          </button>
        {/if}
        <button
          class="btn btn-accent"
          class:danger={$confirmRequest.danger}
          data-confirm
          onclick={() => respond(true)}
        >
          {$confirmRequest.confirmLabel ?? 'Confirm'}
        </button>
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
    border-radius: var(--radius-dialog);
  }
  .dialog h2 {
    font-size: var(--fs-xl);
    margin-bottom: 10px;
  }
  .dialog p {
    font-size: var(--fs-base);
    line-height: 1.5;
    margin: 0 0 18px;
    white-space: pre-line;
  }
  .remember {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: var(--fs-md);
    color: var(--text-muted);
    cursor: pointer;
    margin: 0 0 18px;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
  }
  .actions .danger {
    background: var(--danger-fill);
    border-color: var(--danger-fill);
    color: #fff;
  }
  .actions .danger:hover:not(:disabled) {
    background: var(--danger-fill-hover);
    border-color: var(--danger-fill-hover);
  }
</style>

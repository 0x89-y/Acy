<script lang="ts" module>
  import type { Sun } from '@lucide/svelte';
  export type IconComponent = typeof Sun;
</script>

<script lang="ts">
  import type { Snippet } from 'svelte';

  /**
   * One setting, Windows 11 style: icon, title + description, control on the
   * right, optional content underneath (`children`). `toggle` makes the whole
   * card a <label>, so clicking anywhere flips the switch inside it; only use
   * it when the control is a single checkbox (no other buttons in the card).
   */
  let {
    icon: Icon,
    title,
    description,
    control,
    children,
    toggle = false,
    danger = false,
    titleId
  }: {
    icon?: IconComponent;
    title: string;
    description?: string | Snippet;
    control?: Snippet;
    children?: Snippet;
    toggle?: boolean;
    danger?: boolean;
    titleId?: string;
  } = $props();
</script>

<svelte:element this={toggle ? 'label' : 'div'} class="setting" class:toggle class:danger>
  <div class="row">
    {#if Icon}
      <span class="ic" aria-hidden="true"><Icon size={20} /></span>
    {/if}
    <span class="text">
      <span class="title" id={titleId}>{title}</span>
      {#if typeof description === 'string'}
        <span class="desc">{description}</span>
      {:else if description}
        <span class="desc">{@render description()}</span>
      {/if}
    </span>
    {#if control}
      <div class="control">{@render control()}</div>
    {/if}
  </div>
  {#if children}
    <div class="sub">{@render children()}</div>
  {/if}
</svelte:element>

<style>
  .setting {
    display: block;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface-2);
  }
  .setting.toggle {
    cursor: pointer;
    transition: background 0.15s;
  }
  .setting.toggle:hover {
    background: var(--surface-hover);
  }
  .row {
    display: flex;
    align-items: center;
    gap: 16px;
    min-height: 64px;
    padding: 12px 16px;
  }
  .ic {
    flex-shrink: 0;
    display: inline-flex;
    color: var(--text);
  }
  .danger .ic {
    color: var(--danger);
  }
  .text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .title {
    font-size: var(--fs-base);
    font-weight: 500;
  }
  .desc {
    font-size: var(--fs-sm);
    color: var(--text-muted);
    line-height: 1.45;
    overflow-wrap: anywhere;
  }
  .control {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 8px;
    max-width: 60%;
  }
  .sub {
    padding: 12px 16px 14px 52px;
    border-top: 1px solid var(--border);
  }
  /* Content passed in may be an {#if} that rendered nothing. */
  .sub:not(:has(*)) {
    display: none;
  }
</style>

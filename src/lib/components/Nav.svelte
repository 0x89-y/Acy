<script lang="ts">
  import { Settings } from '@lucide/svelte';
  import WindowControls from './WindowControls.svelte';
</script>

<header class="nav" data-tauri-drag-region>
  <div class="spacer" data-tauri-drag-region></div>
  <div class="right">
    <a href="/settings" class="icon-link" title="Settings" aria-label="Settings">
      <Settings size={18} />
    </a>
    <div class="win-divider"></div>
    <WindowControls />
  </div>
</header>

<style>
  .nav {
    position: sticky;
    top: 0;
    z-index: 20;
    display: flex;
    align-items: center;
    gap: 10px;
    height: 32px;
    padding: 0 4px 0 14px;
    background: var(--surface);
    border-bottom: 1px solid var(--border);
    user-select: none;
  }
  .spacer {
    flex: 1;
  }
  .right {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .win-divider {
    width: 1px;
    height: 20px;
    background: var(--border);
    margin: 0 6px;
  }
  .icon-link {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 26px;
    padding: 0;
    border-radius: var(--radius-sm);
    color: var(--text-muted);
    line-height: 0;
  }
  .icon-link:hover {
    background: var(--surface-hover);
    color: var(--text);
  }
</style>

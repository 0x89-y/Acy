<script lang="ts">
  import { installedLoading, updatesLoading } from '$lib/stores/library';

  let label = $derived(
    $installedLoading
      ? 'Scanning installed apps…'
      : $updatesLoading
        ? 'Checking for updates…'
        : null
  );
</script>

{#if label}
  <div
    class="winget-busy"
    role="status"
    aria-live="polite"
    title="winget is running in the background - other winget actions may wait for it"
  >
    <span class="spinner" aria-hidden="true"></span>
    {label}
  </div>
{/if}

<style>
  .winget-busy {
    position: fixed;
    left: 16px;
    bottom: 16px;
    z-index: 40;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 7px 12px;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    font-size: 0.8rem;
    color: var(--text-muted);
    box-shadow: var(--shadow);
  }
  .spinner {
    width: 12px;
    height: 12px;
    flex-shrink: 0;
    border: 2px solid var(--border-strong);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>

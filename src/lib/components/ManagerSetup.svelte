<script lang="ts">
  import { managers, loadManagers } from '$lib/stores/managers';
  import { settings } from '$lib/stores/settings';
  import { enqueue } from '$lib/stores/ops';
  import * as api from '$lib/api';
  import type { Source } from '$lib/types';

  const names: Record<Source, string> = {
    winget: 'winget',
    scoop: 'Scoop',
    choco: 'Chocolatey',
    msstore: 'Microsoft Store',
    local: 'Local file'
  };

  let busy = $state<Source | null>(null);
  let issues = $derived(
    $managers.filter(
      (m) =>
        m.source !== 'local' &&
        $settings.managers[m.source] !== false &&
        (!m.available || m.needsSetup)
    )
  );

  async function fix(source: Source) {
    busy = source;
    await enqueue(`Set up ${names[source]}`, (opId) => api.bootstrapManager(source, opId));
    busy = null;
    loadManagers(true);
  }
</script>

{#if issues.length > 0}
  <div class="setup card">
    {#each issues as m (m.source)}
      <div class="row">
        <span class="dot" class:warn={m.available} class:off={!m.available}></span>
        <span class="txt">
          <strong>{names[m.source]}</strong>
          <span class="muted">
            {m.detail ?? (m.available ? 'optional setup available' : 'not installed')}
            {#if m.source === 'choco' && !m.available}
              (installing Chocolatey needs an elevated/admin run)
            {/if}
          </span>
        </span>
        <button class="btn" onclick={() => fix(m.source)} disabled={busy === m.source}>
          {busy === m.source ? 'Working…' : m.available ? 'Enhance' : 'Install'}
        </button>
      </div>
    {/each}
  </div>
{/if}

<style>
  .setup {
    padding: 6px 14px;
    margin-bottom: 20px;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 0;
    border-bottom: 1px solid var(--border);
  }
  .row:last-child {
    border-bottom: none;
  }
  .dot {
    width: 9px;
    height: 9px;
    border-radius: 50%;
    flex-shrink: 0;
  }
  .dot.warn {
    background: var(--warning);
  }
  .dot.off {
    background: var(--text-muted);
  }
  .txt {
    display: flex;
    flex-direction: column;
    gap: 1px;
    flex: 1;
    min-width: 0;
  }
  .txt .muted {
    font-size: 0.82rem;
  }
</style>

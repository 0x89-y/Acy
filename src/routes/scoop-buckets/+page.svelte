<script lang="ts">
  import { onMount } from 'svelte';
  import { ArrowLeft } from '@lucide/svelte';
  import * as api from '$lib/api';
  import { enqueue } from '$lib/stores/ops';
  import { confirmAction } from '$lib/stores/confirm';
  import { managers, loadManagers } from '$lib/stores/managers';

  let buckets = $state<string[] | null>(null);
  let knownBuckets = $state<string[]>([]);
  let bucketBusy = $state<string | null>(null);
  let ready = $state(false);
  let scoopAvailable = $derived($managers.find((m) => m.source === 'scoop')?.available ?? false);

  onMount(async () => {
    await loadManagers();
    ready = true;
  });

  async function loadBuckets() {
    try {
      const [b, k] = await Promise.all([api.scoopBuckets(), api.scoopKnownBuckets()]);
      buckets = b;
      knownBuckets = k;
    } catch {
      buckets = [];
    }
  }

  $effect(() => {
    if (scoopAvailable && buckets === null) loadBuckets();
  });

  const BUCKET_INFO: Record<string, string> = {
    main: 'Core command-line tools',
    extras: 'GUI apps - Firefox, VLC, Discord, VS Code…',
    versions: 'Alternate and older app versions',
    nirsoft: 'NirSoft utilities',
    games: 'Games and game tools',
    java: 'Java runtimes and JDKs',
    php: 'PHP versions',
    nonportable: 'Apps that need a full installer',
    sysinternals: 'Microsoft Sysinternals tools'
  };

  let bucketRows = $derived.by(() => {
    const added = buckets ?? [];
    const names = [...new Set([...added, ...knownBuckets])];
    return names
      .map((name) => ({
        name,
        added: added.includes(name),
        description: BUCKET_INFO[name] ?? (added.includes(name) ? 'Added bucket' : '')
      }))
      .sort((a, b) => {
        if (a.name === 'main') return -1;
        if (b.name === 'main') return 1;
        if (a.added !== b.added) return a.added ? -1 : 1;
        return a.name.localeCompare(b.name);
      });
  });

  let addedRows = $derived(bucketRows.filter((r) => r.added));
  let availableRows = $derived(bucketRows.filter((r) => !r.added));

  let selected = $state<'all' | 'added' | 'available'>('all');
  let visibleRows = $derived(
    selected === 'added' ? addedRows : selected === 'available' ? availableRows : bucketRows
  );
  let paneLabel = $derived(
    selected === 'added' ? 'Added' : selected === 'available' ? 'Available' : 'All buckets'
  );
  let loading = $derived(!ready || (scoopAvailable && buckets === null));

  async function addBucket(name: string) {
    bucketBusy = name;
    await enqueue(`Add Scoop bucket: ${name}`, (opId) => api.addScoopBucket(name, opId));
    bucketBusy = null;
    loadBuckets();
  }

  async function removeBucket(name: string) {
    const ok = await confirmAction({
      title: `Remove the "${name}" bucket?`,
      message:
        `Apps you already installed from "${name}" stay, but Scoop won't offer updates ` +
        `for them until you add it back.`,
      confirmLabel: 'Remove bucket',
      danger: true
    });
    if (!ok) return;
    bucketBusy = name;
    await enqueue(`Remove Scoop bucket: ${name}`, (opId) => api.removeScoopBucket(name, opId));
    bucketBusy = null;
    loadBuckets();
  }
</script>

<div class="browse-panel">
  <div class="browse-rail">
    <div class="rail-head">
      <a class="back-btn" href="/settings" title="Back" aria-label="Back"><ArrowLeft size={17} /></a>
      <span class="rail-title">Scoop buckets</span>
    </div>
    <div class="rail-links">
      <button class="rail-link" class:active={selected === 'all'} onclick={() => (selected = 'all')}>
        <span>All buckets</span><span class="rail-count mono">{bucketRows.length}</span>
      </button>
      <button class="rail-link" class:active={selected === 'added'} onclick={() => (selected = 'added')}>
        <span>Added</span><span class="rail-count mono">{addedRows.length}</span>
      </button>
      <button class="rail-link" class:active={selected === 'available'} onclick={() => (selected = 'available')}>
        <span>Available</span><span class="rail-count mono">{availableRows.length}</span>
      </button>
    </div>
  </div>

  <div class="browse-main">
    <div class="pane-head">
      <span class="pane-title">{paneLabel}</span>
      <span class="rail-count mono">{visibleRows.length}</span>
    </div>
    <div class="pane-scroll">
      {#if loading}
        <p class="note muted">Loading…</p>
      {:else if !scoopAvailable}
        <p class="note muted">Scoop isn't installed, so it has no buckets to manage.</p>
      {:else if visibleRows.length === 0}
        <p class="note muted">
          {selected === 'available' ? 'Every known bucket is already added.' : 'No buckets here.'}
        </p>
      {:else}
        <div class="bucket-list">
          {#each visibleRows as row (row.name)}
            <div class="bucket-row" class:is-added={row.added}>
              <div class="bucket-meta">
                <span class="bucket-name mono">{row.name}</span>
                {#if row.description}<span class="bucket-desc muted">{row.description}</span>{/if}
              </div>
              {#if row.added && row.name === 'main'}
                <span class="bucket-state">Added</span>
              {:else if row.added}
                <button
                  class="btn btn-ghost bucket-btn"
                  onclick={() => removeBucket(row.name)}
                  disabled={bucketBusy !== null}
                >
                  {bucketBusy === row.name ? 'Removing…' : 'Remove'}
                </button>
              {:else}
                <button
                  class="btn bucket-btn"
                  onclick={() => addBucket(row.name)}
                  disabled={bucketBusy !== null}
                >
                  {bucketBusy === row.name ? 'Adding…' : 'Add'}
                </button>
              {/if}
            </div>
          {/each}
        </div>
      {/if}
    </div>
  </div>
</div>

<style>
  .browse-panel {
    flex: 1;
    min-height: 0;
    display: flex;
    align-items: stretch;
    overflow: hidden;
    background: var(--surface);
  }
  .browse-rail {
    flex: 0 0 190px;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    border-right: 1px solid var(--border);
    background: var(--surface-2);
  }
  .rail-head {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
  }
  .rail-title {
    font-size: 0.95rem;
    font-weight: 600;
  }
  .back-btn {
    flex-shrink: 0;
    width: 30px;
    height: 30px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface);
    color: var(--text-muted);
    line-height: 0;
    text-decoration: none;
  }
  .back-btn:hover {
    background: var(--surface-hover);
    color: var(--text);
    border-color: var(--accent);
  }
  .rail-links {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
  }
  .rail-link {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    text-align: left;
    padding: 9px 14px;
    border: none;
    border-top: 1px solid var(--border);
    border-left: 2px solid transparent;
    border-radius: 0;
    background: transparent;
    color: var(--text-muted);
    font-size: 0.9rem;
    font-weight: 500;
  }
  .rail-link:first-child {
    border-top: none;
  }
  .rail-link:hover {
    background: var(--surface-hover);
    color: var(--text);
  }
  .rail-link.active {
    background: var(--surface);
    color: var(--text);
    border-left-color: var(--accent);
  }
  .rail-count {
    font-size: 0.72rem;
    color: var(--text-muted);
  }
  .rail-link.active .rail-count {
    color: var(--accent);
  }
  .browse-main {
    flex: 1;
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .pane-head {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 34px;
    padding: 10px 14px;
    border-bottom: 1px solid var(--border);
  }
  .pane-title {
    font-size: 0.95rem;
    font-weight: 600;
  }
  .pane-scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
  }
  .note {
    padding: 20px 16px;
    font-size: 0.9rem;
  }
  .bucket-list {
    display: flex;
    flex-direction: column;
  }
  .bucket-row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 16px;
    border-top: 1px solid var(--border);
  }
  .bucket-row:first-child {
    border-top: none;
  }
  .bucket-row.is-added {
    background: var(--surface-2);
  }
  .bucket-meta {
    display: flex;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
    flex: 1;
  }
  .bucket-name {
    font-size: 0.84rem;
    font-weight: 600;
    color: var(--text);
  }
  .bucket-desc {
    font-size: 0.76rem;
  }
  .bucket-state {
    font-size: 0.76rem;
    font-weight: 600;
    color: var(--accent);
    flex-shrink: 0;
    padding-right: 4px;
  }
  .bucket-btn {
    font-size: 0.8rem;
    padding: 5px 12px;
    flex-shrink: 0;
    min-width: 76px;
  }
</style>

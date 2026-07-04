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
    extras: 'GUI apps — Firefox, VLC, Discord, VS Code…',
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

<a class="back" href="/settings"><ArrowLeft size={16} /> Settings</a>

<header>
  <h1>Scoop buckets</h1>
</header>

{#if !ready}
  <p class="muted">Loading…</p>
{:else if !scoopAvailable}
  <p class="muted">Scoop isn't installed, so it has no buckets to manage.</p>
{:else if buckets === null}
  <p class="muted">Loading…</p>
{:else}
  <div class="bucket-list">
    {#each bucketRows as row (row.name)}
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

<style>
  .back {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    margin-bottom: 18px;
    color: var(--text-muted);
    font-size: 0.9rem;
    text-decoration: none;
  }
  .back:hover {
    color: var(--text);
  }
  header {
    margin-bottom: 20px;
  }
  .bucket-list {
    display: flex;
    flex-direction: column;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    overflow: hidden;
  }
  .bucket-row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 9px 12px;
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

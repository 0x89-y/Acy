<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { ArrowLeft } from '@lucide/svelte';
  import AppCard from '$lib/components/AppCard.svelte';
  import ViewToggle from '$lib/components/ViewToggle.svelte';
  import { curated, loadCurated } from '$lib/stores/curated';
  import { installedKeys, loadInstalled } from '$lib/stores/library';
  import { settings, setDiscoverView } from '$lib/stores/settings';
  import { runOp, summarizeBatch } from '$lib/install';
  import type { CuratedApp, Source, Variant } from '$lib/types';

  let categoryId = $derived(decodeURIComponent($page.params.id ?? ''));
  let category = $derived($curated?.categories.find((cat) => cat.id === categoryId) ?? null);

  function key(source: Source, id: string) {
    return `${source}:${id.toLowerCase()}`;
  }

  function variantsFor(app: CuratedApp): Variant[] {
    const seen = new Set<Source>();
    const variants: Variant[] = [];
    for (const variant of [{ source: app.source, id: app.id }, ...app.alternates]) {
      if ($settings.managers[variant.source] === false || seen.has(variant.source)) continue;
      seen.add(variant.source);
      variants.push(variant);
    }
    return variants;
  }

  function isInstalled(variants: Variant[]) {
    return variants.some((variant) => $installedKeys.has(key(variant.source, variant.id)));
  }

  let apps = $derived((category?.apps ?? []).filter((app) => variantsFor(app).length > 0));
  let layoutClass = $derived($settings.discoverView === 'list' ? 'list-flow' : 'grid');

  let selectMode = $state(false);
  let installing = $state(false);
  let installProgress = $state<{ current: number; total: number; name: string } | null>(null);
  let selectedApps = $state<Map<string, { name: string; variants: Variant[] }>>(new Map());

  function appKey(app: CuratedApp) {
    return `${app.source}:${app.id}`;
  }

  function chosenVariant(variants: Variant[]) {
    return (
      ($settings.preferredSource &&
        variants.find((variant) => variant.source === $settings.preferredSource)) ||
      variants[0]
    );
  }

  function toggleSelected(app: CuratedApp) {
    const variants = variantsFor(app);
    if (variants.length === 0 || isInstalled(variants)) return;
    const appId = appKey(app);
    const next = new Map(selectedApps);
    if (next.has(appId)) next.delete(appId);
    else next.set(appId, { name: app.name ?? app.id, variants });
    selectedApps = next;
  }

  function exitSelect() {
    selectMode = false;
    selectedApps = new Map();
  }

  async function installSelected() {
    installing = true;
    const total = selectedApps.size;
    let completed = 0;
    let current = 0;
    for (const app of selectedApps.values()) {
      current++;
      installProgress = { current, total, name: app.name };
      const variant = chosenVariant(app.variants);
      if (await runOp('install', variant.source, variant.id, app.name)) completed++;
    }
    installProgress = null;
    installing = false;
    exitSelect();
    await loadInstalled(true);
    if (total > 1) summarizeBatch(total, completed, 'installed');
  }

  function onKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape' && selectMode) exitSelect();
  }

  onMount(() => {
    loadCurated();
    loadInstalled();
  });
</script>

<svelte:window onkeydown={onKeydown} />

<a class="back" href="/"><ArrowLeft size={16} /> Discover</a>

{#if !$curated}
  <div class="grid">
    {#each Array(8) as _, i (i)}
      <div class="card skeleton-card" aria-hidden="true"></div>
    {/each}
  </div>
{:else if !category}
  <div class="empty card">
    <h1>Category not found</h1>
    <p class="muted">This category may have been renamed or removed.</p>
    <a class="btn" href="/">Return to Discover</a>
  </div>
{:else}
  <div class="heading">
    <div>
      <h1>{category.title}</h1>
      <span class="muted">{apps.length} {apps.length === 1 ? 'app' : 'apps'}</span>
    </div>
    <button
      class="btn"
      onclick={() => (selectMode ? exitSelect() : (selectMode = true))}
      aria-pressed={selectMode}
    >
      {selectMode ? 'Cancel selection' : 'Select apps'}
    </button>
    <div class="spacer"></div>
    <ViewToggle value={$settings.discoverView} onChange={setDiscoverView} />
  </div>

  {#if selectMode && selectedApps.size > 0}
    <div class="selection-bar">
      <span class="selection-count" role="status" aria-live="polite">
        {#if installProgress}
          Installing {installProgress.current} of {installProgress.total} · {installProgress.name}
        {:else}
          {selectedApps.size} selected
        {/if}
      </span>
      <div class="spacer"></div>
      <button class="btn btn-accent" onclick={installSelected} disabled={installing}>
        {installProgress
          ? `${installProgress.current} of ${installProgress.total}…`
          : `Install ${selectedApps.size}`}
      </button>
    </div>
  {/if}

  {#if apps.length === 0}
    <p class="muted">No apps in this category use an enabled source.</p>
  {:else}
    <div class={layoutClass}>
      {#each apps as app (app.source + app.id)}
        {@const variants = variantsFor(app)}
        {@const installed = isInstalled(variants)}
        <AppCard
          name={app.name ?? app.id}
          description={app.description}
          {variants}
          {installed}
          homepage={app.icon ?? app.homepage}
          tags={app.tags ?? []}
          allowPick
          layout={$settings.discoverView}
          backTo={`/category/${encodeURIComponent(category.id)}`}
          selectable={selectMode && !installed}
          selected={selectedApps.has(appKey(app))}
          onToggleSelect={() => toggleSelected(app)}
          onChanged={() => loadInstalled(true)}
        />
      {/each}
    </div>
  {/if}
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
  .heading {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 18px;
  }
  .heading > div:first-child {
    display: flex;
    align-items: baseline;
    gap: 10px;
  }
  .heading h1 {
    font-size: 1.45rem;
  }
  .heading .muted {
    font-size: 0.82rem;
  }
  .spacer {
    flex: 1;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
    gap: 14px;
  }
  .list-flow {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .selection-bar {
    position: sticky;
    top: 64px;
    z-index: 15;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 14px;
    margin-bottom: 18px;
    background: color-mix(in srgb, var(--surface) 92%, transparent);
    backdrop-filter: blur(8px);
    border: 1px solid var(--accent);
    border-radius: var(--radius);
  }
  .selection-count {
    font-size: 0.9rem;
    font-weight: 600;
  }
  .skeleton-card {
    height: 170px;
    background: var(--surface);
  }
  .empty {
    max-width: 520px;
    margin: 40px auto;
    padding: 36px;
    text-align: center;
  }
  .empty p {
    margin: 8px 0 20px;
  }
  @media (max-width: 700px) {
    .heading {
      flex-wrap: wrap;
    }
    .heading .spacer {
      display: none;
    }
  }
</style>

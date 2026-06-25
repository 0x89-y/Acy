<script lang="ts">
  import AppIcon from './AppIcon.svelte';
  import SourceBadge from './SourceBadge.svelte';
  import InstallButton from './InstallButton.svelte';
  import type { Source } from '$lib/types';

  type Variant = { source: Source; id: string };

  let {
    name,
    description = null,
    variants,
    installed = false,
    sub = null,
    homepage = null,
    onChanged
  }: {
    name: string;
    description?: string | null;
    variants: Variant[];
    installed?: boolean;
    sub?: string | null;
    homepage?: string | null;
    onChanged?: () => void;
  } = $props();

  let primary = $derived(variants[0]);
  let href = $derived(`/app/${primary.source}/${encodeURIComponent(primary.id)}`);
</script>

<div class="card app-card">
  <a class="main" {href}>
    <AppIcon {name} source={primary.source} id={primary.id} {homepage} />
    <div class="meta">
      <div class="name">{name}</div>
      <div class="sub mono">{sub ?? primary.id}</div>
      {#if description}<div class="desc muted">{description}</div>{/if}
    </div>
  </a>
  <div class="foot">
    <div class="badges">
      {#each variants as v (v.source)}<SourceBadge source={v.source} />{/each}
    </div>
    {#if installed}
      <span class="installed">Installed</span>
    {:else}
      <InstallButton source={primary.source} id={primary.id} {name} onDone={onChanged} />
    {/if}
  </div>
</div>

<style>
  .app-card {
    display: flex;
    flex-direction: column;
    padding: 14px;
    gap: 12px;
    transition: border-color 0.15s, box-shadow 0.15s;
  }
  .app-card:hover {
    border-color: var(--border-strong);
    box-shadow: var(--shadow);
  }
  .main {
    display: flex;
    gap: 12px;
    color: inherit;
    text-decoration: none;
    min-width: 0;
  }
  .meta {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .name {
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .sub {
    font-size: 0.72rem;
    color: var(--text-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .desc {
    font-size: 0.82rem;
    margin-top: 4px;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .foot {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    margin-top: auto;
  }
  .badges {
    display: flex;
    gap: 5px;
    flex-wrap: wrap;
  }
  .installed {
    font-size: 0.8rem;
    color: var(--success);
    font-weight: 500;
  }
</style>

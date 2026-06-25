<script lang="ts">
  import { settings } from '$lib/stores/settings';
  import { loadIcon, iconCacheVersion } from '$lib/stores/icons';
  import type { Source } from '$lib/types';

  let {
    name,
    size = 44,
    source = null,
    id = null,
    homepage = null
  }: {
    name: string;
    size?: number;
    source?: Source | null;
    id?: string | null;
    homepage?: string | null;
  } = $props();

  const palette = [
    '#2563eb', '#16a34a', '#9333ea', '#db2777',
    '#ea580c', '#0891b2', '#ca8a04', '#dc2626'
  ];

  function hash(s: string): number {
    let h = 0;
    for (let i = 0; i < s.length; i++) h = (h * 31 + s.charCodeAt(i)) >>> 0;
    return h;
  }

  let initial = $derived((name.trim()[0] ?? '?').toUpperCase());
  let color = $derived(palette[hash(name) % palette.length]);

  let iconUrl = $state<string | null>(null);

  $effect(() => {
    void $iconCacheVersion;
    const enabled = $settings.downloadIcons;
    const s = source;
    const i = id;
    const h = homepage;
    iconUrl = null;
    if (enabled && s && i) {
      loadIcon(s, i, h).then((url) => {
        if (source === s && id === i) iconUrl = url;
      });
    }
  });
</script>

{#if iconUrl}
  <img
    class="icon img"
    src={iconUrl}
    alt=""
    style="width:{size}px; height:{size}px;"
  />
{:else}
  <div
    class="icon"
    style="--c:{color}; width:{size}px; height:{size}px; font-size:{Math.round(size * 0.42)}px;"
  >
    {initial}
  </div>
{/if}

<style>
  .icon {
    display: grid;
    place-items: center;
    border-radius: var(--radius-sm);
    background: color-mix(in srgb, var(--c) 16%, var(--surface));
    color: var(--c);
    font-weight: 600;
    font-family: var(--font-mono);
    flex-shrink: 0;
    border: 1px solid color-mix(in srgb, var(--c) 28%, transparent);
    user-select: none;
  }
  .icon.img {
    object-fit: contain;
    padding: 5px;
    background: var(--surface-2);
    border: 1px solid var(--border);
  }
</style>

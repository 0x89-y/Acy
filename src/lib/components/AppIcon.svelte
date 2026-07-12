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

  // Deterministic avatar palette shared across the cy family (cy-design §8) —
  // same name maps to the same colour on every platform.
  const palette = [
    '#6750a4', '#7d5260', '#1e88e5', '#43a047',
    '#fb8c00', '#8e24aa', '#00897b', '#5e35b1'
  ];

  function hash(s: string): number {
    let h = 0;
    for (let i = 0; i < s.length; i++) h = (h * 31 + s.charCodeAt(i)) >>> 0;
    return h;
  }

  let initial = $derived((name.trim()[0] ?? '?').toUpperCase());
  let color = $derived(palette[hash(name) % palette.length]);

  let iconUrl = $state<string | null>(null);
  let loading = $state(false);

  $effect(() => {
    void $iconCacheVersion; // re-run after the cache is cleared
    const enabled = $settings.downloadIcons;
    const s = source;
    const i = id;
    const h = homepage;
    iconUrl = null;
    if (enabled && s && i) {
      loading = true;
      loadIcon(s, i, h).then((url) => {
        // Ignore if props changed while the request was in flight.
        if (source === s && id === i) {
          iconUrl = url;
          loading = false;
        }
      });
    } else {
      loading = false;
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
{:else if loading}
  <div class="skel skeleton" style="width:{size}px; height:{size}px;"></div>
{:else}
  <div
    class="icon"
    style="--c:{color}; width:{size}px; height:{size}px; font-size:{Math.round(size * 0.42)}px;"
  >
    {initial}
  </div>
{/if}

<style>
  /* Squircle (cy-design §4/§6): solid hashed colour, white glyph. */
  .icon {
    display: grid;
    place-items: center;
    border-radius: 28%;
    background: var(--c);
    color: #fff;
    font-weight: 600;
    font-family: var(--font-mono);
    flex-shrink: 0;
    user-select: none;
  }
  .icon.img {
    object-fit: contain;
    padding: 5px;
    background: var(--surface-2);
    border: 1px solid var(--border);
  }
  .skel {
    flex-shrink: 0;
  }
</style>

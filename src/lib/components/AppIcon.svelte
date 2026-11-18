<script lang="ts">
  import { settings } from '$lib/stores/settings';
  import { requestIcon, iconCacheVersion } from '$lib/stores/icons';
  import { onceVisible } from '$lib/visibility';
  import { steamAppId } from '$lib/installedGroups';
  import type { Source } from '$lib/types';

  let {
    name,
    size = 44,
    source = null,
    id = null,
    homepage = null,
    gameName = null,
    cacheOnly = false
  }: {
    name: string;
    size?: number;
    source?: Source | null;
    id?: string | null;
    homepage?: string | null;
    gameName?: string | null;
    cacheOnly?: boolean;
  } = $props();

  const palette = [
    // Each clears 4.5:1 under the white initial.
    '#6750a4', '#7d5260', '#1565c0', '#2e7d32',
    '#c2410c', '#8e24aa', '#00796b', '#5e35b1'
  ];

  function hash(s: string): number {
    let h = 0;
    for (let i = 0; i < s.length; i++) h = (h * 31 + s.charCodeAt(i)) >>> 0;
    return h;
  }

  let initial = $derived((name.trim()[0] ?? '?').toUpperCase());
  let color = $derived(palette[hash(name) % palette.length]);

  let isGameArt = $derived(
    (source === 'winget' && !!id && steamAppId(id) !== null) || !!gameName
  );

  let iconUrl = $state<string | null>(null);
  let loading = $state(false);
  // Nothing is fetched until the icon is on screen (long Library/search lists).
  let visible = $state(false);

  function watch(node: Element) {
    if (visible) return;
    const stop = onceVisible(node, () => (visible = true));
    return { destroy: stop };
  }

  $effect(() => {
    void $iconCacheVersion;
    const enabled = $settings.downloadIcons;
    const key = $settings.steamGridKey;
    const s = source;
    const i = id;
    const h = homepage;
    const g = gameName;
    const c = cacheOnly;
    iconUrl = null;
    if (!(enabled && s && i)) {
      loading = false;
      return;
    }
    loading = true;
    if (!visible) return;
    // Released when this re-runs or the card unmounts; a fetch nobody waits
    // on any more gets cancelled.
    const req = requestIcon(s, i, h, key, g, c);
    let live = true;
    req.promise.then((url) => {
      if (!live) return;
      iconUrl = url;
      loading = false;
    });
    return () => {
      live = false;
      req.release();
    };
  });
</script>

{#if iconUrl}
  <img
    use:watch
    class="icon img"
    class:cover={isGameArt}
    src={iconUrl}
    alt=""
    style="width:{size}px; height:{size}px;"
  />
{:else if loading}
  <div use:watch class="skel skeleton" style="width:{size}px; height:{size}px;"></div>
{:else}
  <div
    use:watch
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
  .icon.img.cover {
    object-fit: cover;
    padding: 0;
  }
  .skel {
    flex-shrink: 0;
  }
</style>

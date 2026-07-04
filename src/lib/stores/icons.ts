import { invoke } from '@tauri-apps/api/core';
import { writable } from 'svelte/store';
import type { Source } from '$lib/types';

// Bumped when the cache is cleared so icon consumers re-fetch.
export const iconCacheVersion = writable(0);

// Per-session cache of icon lookups so the same app is only requested once.
let cache = new Map<string, Promise<string | null>>();

export function loadIcon(
  source: Source,
  id: string,
  homepage?: string | null
): Promise<string | null> {
  const key = `${source}:${id.toLowerCase()}`;
  let pending = cache.get(key);
  if (!pending) {
    pending = invoke<string | null>('app_icon', {
      source,
      id,
      homepage: homepage ?? null
    }).catch(() => null);
    cache.set(key, pending);
  }
  return pending;
}

export async function clearIconCache(): Promise<void> {
  cache = new Map();
  try {
    await invoke('clear_icon_cache');
  } catch (e) {
    console.error('clear icon cache failed', e);
  }
  iconCacheVersion.update((n) => n + 1);
}

/**
 * Drop the per-session lookup map and bump the version so icon consumers
 * re-request — without wiping the on-disk cache. Use after re-fetching missing
 * icons so the ones that just downloaded show up.
 */
export function refreshIcons(): void {
  cache = new Map();
  iconCacheVersion.update((n) => n + 1);
}

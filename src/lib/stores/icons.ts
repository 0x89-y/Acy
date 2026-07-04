import { invoke } from '@tauri-apps/api/core';
import { writable } from 'svelte/store';
import type { Source } from '$lib/types';

export const iconCacheVersion = writable(0);

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

export function refreshIcons(): void {
  cache = new Map();
  iconCacheVersion.update((n) => n + 1);
}

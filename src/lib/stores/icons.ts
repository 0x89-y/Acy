import { invoke } from '@tauri-apps/api/core';
import { writable, get } from 'svelte/store';
import type { Source } from '$lib/types';
import * as api from '$lib/api';
import { settings } from '$lib/stores/settings';
import { installed } from '$lib/stores/library';
import { bucketKey } from '$lib/installedGroups';

// Bumped when the cache is cleared so icon consumers re-fetch.
export const iconCacheVersion = writable(0);

// Progress of a "re-download missing icons" run. Lives in the store (not the
// Settings component) so it keeps running - and stays visible - when the user
// navigates out of and back into Settings.
export type IconRefetchState =
  | { status: 'idle' }
  | { status: 'running'; current: number; total: number }
  | { status: 'done'; message: string };

export const iconRefetch = writable<IconRefetchState>({ status: 'idle' });

let refetchInFlight = false;

/**
 * Re-fetch icons for every app that's missing one - curated apps plus every
 * installed app Acy classifies as a game (so Steam + other launcher games get
 * retried without wiping the cache). Skips ones already cached; gentle, one at a
 * time, so it doesn't re-hit the favicon-service rate limit. Safe to call while
 * a run is already in flight (it's a no-op then).
 */
export async function refetchMissingIcons(): Promise<void> {
  if (refetchInFlight) return;
  refetchInFlight = true;
  iconRefetch.set({ status: 'running', current: 0, total: 0 });
  const unlisten = await api.onIconRefetchProgress((p) =>
    iconRefetch.set({ status: 'running', current: p.current, total: p.total })
  );
  try {
    const file = await api.getCurated();
    const curatedItems = file.categories.flatMap((c) =>
      c.apps.map((a) => ({
        source: a.source,
        id: a.id,
        homepage: a.icon ?? a.homepage ?? null,
        gameName: null as string | null
      }))
    );
    // Only installed apps Acy classifies as games - their name drives the
    // SteamGridDB-by-name match. (Other installed apps are left for later.)
    const installedGames = get(installed)
      .filter((p) => bucketKey(p) === 'games')
      .map((p) => ({
        source: p.source,
        id: p.id,
        homepage: p.homepage ?? null,
        gameName: p.name as string | null
      }));
    // Dedupe by source:id so an app in both lists is only fetched once.
    const byKey = new Map<string, (typeof curatedItems)[number]>();
    for (const it of [...curatedItems, ...installedGames]) {
      const k = `${it.source}:${it.id.toLowerCase()}`;
      if (!byKey.has(k)) byKey.set(k, it);
    }
    const items = [...byKey.values()];
    const { fetched, failed } = await api.refetchMissingIcons(items, get(settings).steamGridKey);
    refreshIcons();
    let message: string;
    if (fetched === 0 && failed === 0) message = 'No missing icons.';
    else if (failed === 0)
      message = `Downloaded ${fetched} missing ${fetched === 1 ? 'icon' : 'icons'}.`;
    else message = `Downloaded ${fetched}, ${failed} still unavailable.`;
    iconRefetch.set({ status: 'done', message });
  } catch (e) {
    console.error('refetch missing icons failed', e);
    iconRefetch.set({ status: 'done', message: 'Could not re-download icons.' });
  } finally {
    unlisten();
    refetchInFlight = false;
  }
}

// Per-session cache of icon lookups so the same app is only requested once.
let cache = new Map<string, Promise<string | null>>();

export function loadIcon(
  source: Source,
  id: string,
  homepage?: string | null,
  steamGridKey?: string | null,
  gameName?: string | null
): Promise<string | null> {
  const key = `${source}:${id.toLowerCase()}`;
  let pending = cache.get(key);
  if (!pending) {
    pending = invoke<string | null>('app_icon', {
      source,
      id,
      homepage: homepage ?? null,
      steamgridKey: steamGridKey || null,
      gameName: gameName || null
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
 * re-request - without wiping the on-disk cache. Use after re-fetching missing
 * icons so the ones that just downloaded show up.
 */
export function refreshIcons(): void {
  cache = new Map();
  iconCacheVersion.update((n) => n + 1);
}

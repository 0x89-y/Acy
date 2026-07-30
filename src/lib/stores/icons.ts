import { invoke } from '@tauri-apps/api/core';
import { writable, get } from 'svelte/store';
import type { Source } from '$lib/types';
import * as api from '$lib/api';
import { settings } from '$lib/stores/settings';
import { installed } from '$lib/stores/library';
import { bucketKey } from '$lib/installedGroups';

export const iconCacheVersion = writable(0);

export type IconRefetchState =
  | { status: 'idle' }
  | { status: 'running'; current: number; total: number }
  | { status: 'done'; message: string };

export const iconRefetch = writable<IconRefetchState>({ status: 'idle' });

let refetchInFlight = false;

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
    const installedGames = get(installed)
      .filter((p) => bucketKey(p) === 'games')
      .map((p) => ({
        source: p.source,
        id: p.id,
        homepage: p.homepage ?? null,
        gameName: p.name as string | null
      }));
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

function setIcon(source: Source, id: string, url: string | null): void {
  cache.set(`${source}:${id.toLowerCase()}`, Promise.resolve(url));
  iconCacheVersion.update((n) => n + 1);
}

export async function redownloadIcon(
  source: Source,
  id: string,
  homepage?: string | null,
  gameName?: string | null
): Promise<string | null> {
  const url = await api.refreshAppIcon(source, id, homepage, get(settings).steamGridKey, gameName);
  setIcon(source, id, url);
  return url;
}

export async function deleteIcon(source: Source, id: string): Promise<void> {
  await api.deleteAppIcon(source, id);
  setIcon(source, id, null);
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

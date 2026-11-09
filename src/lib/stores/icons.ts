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

// A full (fetching) load. `users` counts the cards waiting on it; when that
// drops to zero before it settles, the backend fetch is cancelled and the entry
// forgotten, so the next card to want the icon starts over. `pinned` loads
// (callers that never release) are never cancelled.
type Entry = {
  promise: Promise<string | null>;
  ticket: number;
  done: boolean;
  users: number;
  pinned: boolean;
};

let cache = new Map<string, Entry>();
// Separate memo for disk-only lookups: a "not cached" null here must not stop a
// later full load (e.g. the app page) from fetching the icon.
let cachedOnly = new Map<string, Promise<string | null>>();
let nextTicket = 1;

const keyOf = (source: Source, id: string) => `${source}:${id.toLowerCase()}`;

function settledEntry(url: string | null): Entry {
  return { promise: Promise.resolve(url), ticket: 0, done: true, users: 0, pinned: true };
}

function fullEntry(
  source: Source,
  id: string,
  homepage?: string | null,
  steamGridKey?: string | null,
  gameName?: string | null
): Entry {
  const key = keyOf(source, id);
  const existing = cache.get(key);
  if (existing) return existing;
  const entry: Entry = {
    promise: Promise.resolve(null),
    ticket: nextTicket++,
    done: false,
    users: 0,
    pinned: false
  };
  entry.promise = invoke<api.IconRef | null>('app_icon', {
    source,
    id,
    homepage: homepage ?? null,
    steamgridKey: steamGridKey || null,
    gameName: gameName || null,
    ticket: entry.ticket
  })
    .then(api.iconSrc, () => null)
    .finally(() => {
      entry.done = true;
    });
  cache.set(key, entry);
  return entry;
}

export type IconRequest = { promise: Promise<string | null>; release: () => void };

/** Load an icon for a card. Call `release` when the card no longer needs it
 * (unmount, or its source/id changed): an unwanted fetch is cancelled. */
export function requestIcon(
  source: Source,
  id: string,
  homepage?: string | null,
  steamGridKey?: string | null,
  gameName?: string | null,
  cacheOnly = false
): IconRequest {
  const key = keyOf(source, id);
  if (cacheOnly && !cache.has(key)) {
    let hit = cachedOnly.get(key);
    if (!hit) {
      hit = invoke<api.IconRef | null>('app_icon', { source, id, cacheOnly: true }).then(
        api.iconSrc,
        () => null
      );
      cachedOnly.set(key, hit);
    }
    return { promise: hit, release: () => {} };
  }
  const entry = fullEntry(source, id, homepage, steamGridKey, gameName);
  entry.users++;
  let released = false;
  return {
    promise: entry.promise,
    release: () => {
      if (released) return;
      released = true;
      entry.users--;
      // Deferred: an effect re-run releases and re-requests the same icon in
      // one go, which must not cancel it.
      setTimeout(() => {
        if (entry.users > 0 || entry.done || entry.pinned || cache.get(key) !== entry) return;
        cache.delete(key);
        invoke('cancel_icon', { ticket: entry.ticket }).catch(() => {});
      }, 0);
    }
  };
}

/** Load an icon without ever cancelling it (e.g. the icon editor). */
export function loadIcon(
  source: Source,
  id: string,
  homepage?: string | null,
  steamGridKey?: string | null,
  gameName?: string | null
): Promise<string | null> {
  const entry = fullEntry(source, id, homepage, steamGridKey, gameName);
  entry.pinned = true;
  return entry.promise;
}

function setIcon(source: Source, id: string, url: string | null): void {
  cache.set(keyOf(source, id), settledEntry(url));
  cachedOnly.delete(keyOf(source, id));
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
  cachedOnly = new Map();
  try {
    await invoke('clear_icon_cache');
  } catch (e) {
    console.error('clear icon cache failed', e);
  }
  iconCacheVersion.update((n) => n + 1);
}

export function refreshIcons(): void {
  cache = new Map();
  cachedOnly = new Map();
  iconCacheVersion.update((n) => n + 1);
}

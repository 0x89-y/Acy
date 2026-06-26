import { writable, derived, get } from 'svelte/store';
import * as api from '$lib/api';
import type { Package, Source } from '$lib/types';
import { enabledSources, updatesCount } from './managers';

// Caches the installed apps and available updates so navigating back to the
// Installed page is instant. Each list reloads only when forced (Refresh, or
// after an install/uninstall/update) or when the set of enabled sources
// changes.
//
// The lists are also persisted to localStorage and hydrated on startup, so a
// cold launch shows the last known installed apps (and curated install marks)
// instantly while a fresh scan runs in the background — stale-while-revalidate.
// We deliberately leave the in-memory signatures empty after hydrating, so the
// first load() of the session always revalidates against the live managers.

const INSTALLED_KEY = 'acy-installed-cache';
const UPDATES_KEY = 'acy-updates-cache';

function sig(sources: Source[]): string {
  return [...sources].sort().join(',');
}

function readCache(key: string): Package[] | null {
  if (typeof localStorage === 'undefined') return null;
  try {
    const raw = localStorage.getItem(key);
    const parsed = raw ? JSON.parse(raw) : null;
    return Array.isArray(parsed) ? (parsed as Package[]) : null;
  } catch {
    return null;
  }
}

function writeCache(key: string, value: Package[]): void {
  if (typeof localStorage === 'undefined') return;
  try {
    localStorage.setItem(key, JSON.stringify(value));
  } catch {
    // ignore quota / serialization errors — the cache is best-effort
  }
}

// ---- Installed apps ----

const cachedInstalled = readCache(INSTALLED_KEY);

export const installed = writable<Package[]>(cachedInstalled ?? []);
export const installedLoading = writable(false);
export const installedError = writable<string | null>(null);
export const installedReady = writable(cachedInstalled !== null);

let installedSig = '';

export async function loadInstalled(force = false): Promise<void> {
  const sources = get(enabledSources);
  const current = sig(sources);
  if (!force && get(installedReady) && current === installedSig) return;
  if (get(installedLoading)) return;

  installedLoading.set(true);
  installedError.set(null);
  try {
    const list = await api.listInstalled(sources);
    installed.set(list);
    writeCache(INSTALLED_KEY, list);
    installedSig = current;
    installedReady.set(true);
  } catch (e) {
    installedError.set(String(e));
  } finally {
    installedLoading.set(false);
  }
}

// ---- Available updates ----

const cachedUpdates = readCache(UPDATES_KEY);

export const updates = writable<Package[]>(cachedUpdates ?? []);
export const updatesLoading = writable(false);
export const updatesError = writable<string | null>(null);
export const updatesReady = writable(cachedUpdates !== null);

let updatesSig = '';

if (cachedUpdates) updatesCount.set(cachedUpdates.length);

export async function loadUpdates(force = false): Promise<void> {
  const sources = get(enabledSources);
  const current = sig(sources);
  if (!force && get(updatesReady) && current === updatesSig) return;
  if (get(updatesLoading)) return;

  updatesLoading.set(true);
  updatesError.set(null);
  try {
    const list = await api.listUpdates(sources);
    updates.set(list);
    writeCache(UPDATES_KEY, list);
    updatesCount.set(list.length);
    updatesSig = current;
    updatesReady.set(true);
  } catch (e) {
    updatesError.set(String(e));
  } finally {
    updatesLoading.set(false);
  }
}

/** Set of `source:id` keys for fast "is this installed?" lookups on cards. */
export const installedKeys = derived(
  installed,
  ($installed) => new Set($installed.map((p) => `${p.source}:${p.id.toLowerCase()}`))
);

/** Force a reload of both lists (after an install/uninstall/update). */
export function refreshLibrary(): Promise<void[]> {
  return Promise.all([loadInstalled(true), loadUpdates(true)]);
}

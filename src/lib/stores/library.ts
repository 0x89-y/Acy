import { writable, derived, get } from 'svelte/store';
import * as api from '$lib/api';
import type { Package, Source } from '$lib/types';
import { enabledSources, updatesCount } from './managers';
import { ignoredUpdateKeys, packageIgnoredKey } from './ignoredUpdates';
import { settings } from './settings';

const INSTALLED_KEY = 'acy-installed-cache';
const UPDATES_KEY = 'acy-updates-cache';
const CHECKED_KEY = 'acy-last-checked';

function sig(sources: Source[]): string {
  return [...sources].sort().join(',');
}

export function isAcyPackage(p: Package): boolean {
  if (p.source !== 'winget') return false;
  if (p.name.trim().toLowerCase() === 'acy') return true;
  const id = p.id.toLowerCase();
  if (id.endsWith('\\acy') || id.endsWith('/acy')) return true;
  const loc = (p.installLocation ?? '').replace(/[\\/]+$/, '').toLowerCase();
  return loc.endsWith('\\acy') || loc.endsWith('/acy');
}

function withoutSelf(list: Package[]): Package[] {
  return list.filter((p) => !isAcyPackage(p));
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
  }
}

const cachedInstalled = readCache(INSTALLED_KEY);

export const installed = writable<Package[]>(cachedInstalled ? withoutSelf(cachedInstalled) : []);
export const installedLoading = writable(false);
export const installedError = writable<string | null>(null);
export const installedReady = writable(cachedInstalled !== null);

function readChecked(): number | null {
  if (typeof localStorage === 'undefined') return null;
  const raw = Number(localStorage.getItem(CHECKED_KEY));
  return Number.isFinite(raw) && raw > 0 ? raw : null;
}
export const lastChecked = writable<number | null>(readChecked());

let installedSig = '';

export async function loadInstalled(force = false): Promise<void> {
  const sources = get(enabledSources);
  const current = sig(sources);
  if (get(installedLoading)) return;
  if (!force && get(installedReady) && current === installedSig) return;

  if (sources.includes('winget') && (force || !get(installedReady))) {
    try {
      const fast = withoutSelf(await api.listInstalledFast());
      if (fast.length > 0) {
        installed.set(fast);
        installedReady.set(true);
      }
    } catch {
    }
  }

  if (!force && !get(settings).refreshOnStartup) return;

  installedLoading.set(true);
  installedError.set(null);
  try {
    const list = withoutSelf(await api.listInstalled(sources));
    installed.set(list);
    writeCache(INSTALLED_KEY, list);
    installedSig = current;
    installedReady.set(true);
    const now = Date.now();
    lastChecked.set(now);
    if (typeof localStorage !== 'undefined') localStorage.setItem(CHECKED_KEY, String(now));
  } catch (e) {
    installedError.set(String(e));
  } finally {
    installedLoading.set(false);
  }
}

const cachedUpdates = readCache(UPDATES_KEY);

export const updates = writable<Package[]>(cachedUpdates ? withoutSelf(cachedUpdates) : []);
export const updatesLoading = writable(false);
export const updatesError = writable<string | null>(null);
export const updatesReady = writable(cachedUpdates !== null);

let updatesSig = '';

export const actionableUpdates = derived(
  [updates, ignoredUpdateKeys],
  ([$updates, $ignored]) => $updates.filter((p) => !$ignored.has(packageIgnoredKey(p)))
);

export const ignoredUpdates = derived(
  [updates, ignoredUpdateKeys],
  ([$updates, $ignored]) => $updates.filter((p) => $ignored.has(packageIgnoredKey(p)))
);

actionableUpdates.subscribe((list) => updatesCount.set(list.length));

export async function loadUpdates(force = false): Promise<void> {
  const sources = get(enabledSources);
  const current = sig(sources);
  if (!force && get(updatesReady) && current === updatesSig) return;
  if (get(updatesLoading)) return;
  if (!force && !get(settings).refreshOnStartup) return;

  updatesLoading.set(true);
  updatesError.set(null);
  try {
    const list = withoutSelf(await api.listUpdates(sources));
    updates.set(list);
    writeCache(UPDATES_KEY, list);
    updatesSig = current;
    updatesReady.set(true);
  } catch (e) {
    updatesError.set(String(e));
  } finally {
    updatesLoading.set(false);
  }
}

export const installedKeys = derived(
  installed,
  ($installed) => new Set($installed.map((p) => `${p.source}:${p.id.toLowerCase()}`))
);

export function refreshLibrary(): Promise<void[]> {
  return Promise.all([loadInstalled(true), loadUpdates(true)]);
}

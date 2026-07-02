import { writable, derived, get } from 'svelte/store';
import * as api from '$lib/api';
import type { Package, Source } from '$lib/types';
import { enabledSources, updatesCount } from './managers';
import { ignoredUpdateKeys, packageIgnoredKey } from './ignoredUpdates';
import { settings } from './settings';

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
const CHECKED_KEY = 'acy-last-checked';

function sig(sources: Source[]): string {
  return [...sources].sort().join(',');
}

// Acy installs into Windows' ARP, so winget lists Acy itself (id like
// `ARP\User\X64\Acy`). The Installed page already shows a dedicated "current
// app" row and self-update is handled separately, so drop Acy's own package
// from the managed lists to avoid a duplicate entry / update — and, critically,
// so Acy can never be uninstalled/switched through its own UI. Matched broadly
// (name, id, or install folder) so it can't slip through and remove itself.
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
    // ignore quota / serialization errors — the cache is best-effort
  }
}

// ---- Installed apps ----

const cachedInstalled = readCache(INSTALLED_KEY);

export const installed = writable<Package[]>(cachedInstalled ? withoutSelf(cachedInstalled) : []);
export const installedLoading = writable(false);
export const installedError = writable<string | null>(null);
export const installedReady = writable(cachedInstalled !== null);

/** Epoch ms of the last successful installed-apps scan (null = never). */
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

  // Instant registry paint so the list isn't blank on a fresh launch / forced
  // refresh. This touches only the registry — no winget, no lock, no scanning
  // indicator — so it's safe to always do.
  if (sources.includes('winget') && (force || !get(installedReady))) {
    try {
      const fast = withoutSelf(await api.listInstalledFast());
      if (fast.length > 0) {
        installed.set(fast);
        installedReady.set(true);
      }
    } catch {
      // ignore — the authoritative pass (if it runs) is what matters
    }
  }

  // The slow, authoritative winget/managers scan runs only on an explicit
  // Refresh, or on startup when the user hasn't disabled it. Otherwise the
  // cached/registry list stands and the winget lock stays free (so installs and
  // uninstalls aren't blocked behind it).
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

// ---- Available updates ----

const cachedUpdates = readCache(UPDATES_KEY);

export const updates = writable<Package[]>(cachedUpdates ? withoutSelf(cachedUpdates) : []);
export const updatesLoading = writable(false);
export const updatesError = writable<string | null>(null);
export const updatesReady = writable(cachedUpdates !== null);

let updatesSig = '';

/** Updates the user has not ignored for this specific available version. */
export const actionableUpdates = derived(
  [updates, ignoredUpdateKeys],
  ([$updates, $ignored]) => $updates.filter((p) => !$ignored.has(packageIgnoredKey(p)))
);

/** Ignored updates are retained so the Installed page can restore them. */
export const ignoredUpdates = derived(
  [updates, ignoredUpdateKeys],
  ([$updates, $ignored]) => $updates.filter((p) => $ignored.has(packageIgnoredKey(p)))
);

// The nav badge, tray tooltip, and update notifications count actionable
// updates only. This also reacts immediately when an update is ignored.
actionableUpdates.subscribe((list) => updatesCount.set(list.length));

export async function loadUpdates(force = false): Promise<void> {
  const sources = get(enabledSources);
  const current = sig(sources);
  if (!force && get(updatesReady) && current === updatesSig) return;
  if (get(updatesLoading)) return;
  // Skip the winget update check on launch when the user disabled startup
  // refresh — regardless of whether anything is cached.
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

/** Set of `source:id` keys for fast "is this installed?" lookups on cards. */
export const installedKeys = derived(
  installed,
  ($installed) => new Set($installed.map((p) => `${p.source}:${p.id.toLowerCase()}`))
);

/** Force a reload of both lists (after an install/uninstall/update). */
export function refreshLibrary(): Promise<void[]> {
  return Promise.all([loadInstalled(true), loadUpdates(true)]);
}

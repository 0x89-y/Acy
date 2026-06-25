import { writable, derived, get } from 'svelte/store';
import * as api from '$lib/api';
import type { Package, Source } from '$lib/types';
import { enabledSources, updatesCount } from './managers';

function sig(sources: Source[]): string {
  return [...sources].sort().join(',');
}

export const installed = writable<Package[]>([]);
export const installedLoading = writable(false);
export const installedError = writable<string | null>(null);
export const installedReady = writable(false);

let installedSig = '';

export async function loadInstalled(force = false): Promise<void> {
  const sources = get(enabledSources);
  const current = sig(sources);
  if (!force && get(installedReady) && current === installedSig) return;
  if (get(installedLoading)) return;

  installedLoading.set(true);
  installedError.set(null);
  try {
    installed.set(await api.listInstalled(sources));
    installedSig = current;
    installedReady.set(true);
  } catch (e) {
    installedError.set(String(e));
  } finally {
    installedLoading.set(false);
  }
}

export const updates = writable<Package[]>([]);
export const updatesLoading = writable(false);
export const updatesError = writable<string | null>(null);
export const updatesReady = writable(false);

let updatesSig = '';

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
    updatesCount.set(list.length);
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

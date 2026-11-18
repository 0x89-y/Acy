import { writable, derived, get } from 'svelte/store';
import type { Variant } from '$lib/types';
import { runOp, summarizeBatch } from '$lib/install';
import { settings } from './settings';
import { loadInstalled } from './library';

/** An app waiting in the install queue (the tray at the bottom of the window). */
export interface QueuedApp {
  key: string;
  name: string;
  /** Install options, primary first; the preferred source wins when installing. */
  variants: Variant[];
}

export const installQueue = writable<QueuedApp[]>([]);
export const queuedKeys = derived(installQueue, ($q) => new Set($q.map((a) => a.key)));

/** Progress of the install batch the tray started, if one is running. */
export const batch = writable<{ current: number; total: number; name: string } | null>(null);

export function queueApps(apps: QueuedApp[]) {
  installQueue.update((q) => {
    const have = new Set(q.map((a) => a.key));
    return [...q, ...apps.filter((a) => !have.has(a.key) && have.add(a.key))];
  });
}

export function unqueueApp(key: string) {
  installQueue.update((q) => q.filter((a) => a.key !== key));
}

export function toggleQueued(app: QueuedApp) {
  if (get(queuedKeys).has(app.key)) unqueueApp(app.key);
  else queueApps([app]);
}

export function clearQueue() {
  installQueue.set([]);
}

function chosenVariant(vs: Variant[]): Variant {
  const pref = get(settings).preferredSource;
  return (pref && vs.find((v) => v.source === pref)) || vs[0];
}

/**
 * Installs apps one after another (Windows Installer can't run two at once).
 * Each still becomes its own operation with live output in the tray.
 */
export async function installApps(apps: QueuedApp[]): Promise<void> {
  if (apps.length === 0 || get(batch)) return;
  let ok = 0;
  for (const [i, app] of apps.entries()) {
    batch.set({ current: i + 1, total: apps.length, name: app.name });
    const v = chosenVariant(app.variants);
    if (await runOp('install', v.source, v.id, app.name)) ok++;
  }
  batch.set(null);
  loadInstalled(true);
  if (apps.length > 1) summarizeBatch(apps.length, ok, 'installed');
}

/** Installs everything in the queue and empties it. */
export function installQueued(): Promise<void> {
  const apps = get(installQueue);
  clearQueue();
  return installApps(apps);
}

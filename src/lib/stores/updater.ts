import { writable, get } from 'svelte/store';
import { check, type Update } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import * as api from '$lib/api';
import { settings } from './settings';

// Drives the app's own self-update: a manual check that asks before downloading,
// plus an optional background check (on startup + periodic) that only notifies.

export type UpdaterPhase = 'idle' | 'checking' | 'available' | 'uptodate' | 'downloading' | 'error';

export const updaterPhase = writable<UpdaterPhase>('idle');
export const updaterVersion = writable<string | null>(null);
export const updaterError = writable<string | null>(null);

/** The found-but-not-yet-installed update, kept so the user can confirm download. */
let pending: Update | null = null;

/** How often the background check runs when auto-check is enabled. */
const CHECK_INTERVAL_MS = 3 * 60 * 60 * 1000; // 3 hours

async function runCheck(silent: boolean) {
  const phase = get(updaterPhase);
  if (phase === 'checking' || phase === 'downloading') return;
  if (!silent) updaterPhase.set('checking');
  updaterError.set(null);
  try {
    const update = await check();
    if (update) {
      pending = update;
      updaterVersion.set(update.version);
      updaterPhase.set('available');
      if (silent) {
        api
          .notify('Update available', `Acy ${update.version} is ready to install.`)
          .catch(() => {});
      }
    } else if (!silent) {
      pending = null;
      updaterVersion.set(null);
      updaterPhase.set('uptodate');
    }
  } catch (e) {
    if (!silent) {
      updaterError.set(String(e));
      updaterPhase.set('error');
    }
  }
}

/** Manual check from Settings - surfaces checking / up-to-date / available. */
export const checkForUpdate = () => runCheck(false);

/** Silent background check; only notifies (and arms the download) if found. */
export const backgroundCheck = () => runCheck(true);

/** Download + install the pending update, then relaunch. Called on user confirm. */
export async function installUpdate() {
  if (!pending) return;
  updaterPhase.set('downloading');
  updaterError.set(null);
  try {
    await pending.downloadAndInstall();
    await relaunch();
  } catch (e) {
    updaterError.set(String(e));
    updaterPhase.set('error');
  }
}

let started = false;

/** Set up startup + periodic background checks (gated by the auto-check setting). */
export function initAppUpdater() {
  if (started) return;
  started = true;
  if (get(settings).autoCheckUpdates) backgroundCheck();
  setInterval(() => {
    if (get(settings).autoCheckUpdates) backgroundCheck();
  }, CHECK_INTERVAL_MS);
}

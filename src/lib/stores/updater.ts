import { writable, get } from 'svelte/store';
import { check, type Update } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import * as api from '$lib/api';
import { settings } from './settings';

export type UpdaterPhase = 'idle' | 'checking' | 'available' | 'uptodate' | 'downloading' | 'error';

export const updaterPhase = writable<UpdaterPhase>('idle');
export const updaterVersion = writable<string | null>(null);
export const updaterError = writable<string | null>(null);

let pending: Update | null = null;

const CHECK_INTERVAL_MS = 3 * 60 * 60 * 1000;

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

export const checkForUpdate = () => runCheck(false);

export const backgroundCheck = () => runCheck(true);

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

export function initAppUpdater() {
  if (started) return;
  started = true;
  if (get(settings).autoCheckUpdates) backgroundCheck();
  setInterval(() => {
    if (get(settings).autoCheckUpdates) backgroundCheck();
  }, CHECK_INTERVAL_MS);
}

import { get, writable } from 'svelte/store';
import { getCurrentWindow } from '@tauri-apps/api/window';
import * as api from '$lib/api';
import { updatesCount } from './managers';
import { settings } from './settings';

export const closePromptOpen = writable(false);

let started = false;
let lastNotified = 0;

export async function initTray() {
  if (started) return;
  started = true;

  try {
    const win = getCurrentWindow();
    await win.onCloseRequested((event) => {
      const s = get(settings);
      if (s.closeToTray) {
        event.preventDefault();
        void win.hide();
      } else if (s.askCloseToTray) {
        event.preventDefault();
        closePromptOpen.set(true);
      }
    });
  } catch (e) {
    console.error('tray close handler init failed', e);
  }

  updatesCount.subscribe((count) => {
    api.setUpdateCount(count).catch(() => {});
    if (get(settings).notifyUpdates && count > lastNotified) {
      api
        .notify(
          'Updates available',
          `${count} update${count === 1 ? '' : 's'} ready to install in Acy.`
        )
        .catch(() => {});
    }
    lastNotified = count;
  });
}

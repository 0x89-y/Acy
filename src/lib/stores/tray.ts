import { get } from 'svelte/store';
import { getCurrentWindow } from '@tauri-apps/api/window';
import * as api from '$lib/api';
import { updatesCount } from './managers';
import { settings } from './settings';

// Wires the running app to the system tray:
//  - the close (✕) button hides to the tray instead of quitting, when enabled;
//  - the tray tooltip mirrors the available-update count;
//  - an optional desktop notification fires when new updates appear.

let started = false;
/** Highest count we've already notified about, to avoid repeat alerts. */
let lastNotified = 0;

export async function initTray() {
  if (started) return;
  started = true;

  // Intercept the window close so it can hide to the tray when the user opts in.
  try {
    const win = getCurrentWindow();
    await win.onCloseRequested((event) => {
      if (get(settings).closeToTray) {
        event.preventDefault();
        void win.hide();
      }
    });
  } catch (e) {
    console.error('tray close handler init failed', e);
  }

  // Keep the tray tooltip current, and notify on a rise in the update count.
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

import { writable, derived } from 'svelte/store';
import { detectManagers } from '$lib/api';
import type { ManagerStatus, Manager } from '$lib/types';
import { settings } from './settings';

const ALL_SOURCES: Manager[] = ['winget', 'scoop', 'choco', 'msstore'];

export const managers = writable<ManagerStatus[]>([]);
/** Number of available updates, surfaced as a nav badge. */
export const updatesCount = writable<number>(0);

/**
 * Sources actually used for search / installed / updates: enabled in settings
 * AND available on the machine. Before detection runs we don't yet know
 * availability, so enabled sources are assumed usable.
 */
export const enabledSources = derived([managers, settings], ([$managers, $settings]) => {
  const detected = $managers.length > 0;
  const available = new Map($managers.map((m) => [m.source, m.available]));
  return ALL_SOURCES.filter(
    (s) => $settings.managers[s] !== false && (!detected || available.get(s) === true)
  );
});

let loaded = false;

export async function loadManagers(force = false) {
  if (loaded && !force) return;
  loaded = true;
  try {
    managers.set(await detectManagers());
  } catch (e) {
    console.error('manager detection failed', e);
    loaded = false; // allow a retry
  }
}

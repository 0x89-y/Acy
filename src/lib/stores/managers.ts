import { writable, derived } from 'svelte/store';
import { detectManagers } from '$lib/api';
import type { ManagerStatus, Source } from '$lib/types';
import { settings } from './settings';

const ALL_SOURCES: Source[] = ['winget', 'scoop', 'choco'];

export const managers = writable<ManagerStatus[]>([]);
export const updatesCount = writable<number>(0);

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
    loaded = false;
  }
}

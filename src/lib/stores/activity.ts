import { writable } from 'svelte/store';
import type { Source } from '$lib/types';

// A local history of the actions the user took (install / update / uninstall),
// persisted to localStorage so it survives restarts. Surfaced in Settings as a
// personal activity log alongside the app's own changelog.

export type ActivityAction = 'install' | 'update' | 'uninstall' | 'update-all' | 'setup';

export interface ActivityEntry {
  id: string;
  action: ActivityAction;
  /** Human label: app name, or "winget" for an update-all. */
  name: string;
  source?: Source;
  ok: boolean;
  /** Epoch milliseconds. */
  at: number;
}

const KEY = 'acy-activity';
/** Cap the log so localStorage can't grow without bound. */
const MAX = 200;

function newId(): string {
  if (typeof crypto !== 'undefined' && 'randomUUID' in crypto) return crypto.randomUUID();
  return `act-${Date.now()}-${Math.random().toString(16).slice(2)}`;
}

function load(): ActivityEntry[] {
  if (typeof localStorage === 'undefined') return [];
  try {
    const raw = localStorage.getItem(KEY);
    return raw ? (JSON.parse(raw) as ActivityEntry[]) : [];
  } catch {
    return [];
  }
}

export const activity = writable<ActivityEntry[]>(load());

activity.subscribe((list) => {
  if (typeof localStorage !== 'undefined') {
    localStorage.setItem(KEY, JSON.stringify(list));
  }
});

export function logActivity(entry: Omit<ActivityEntry, 'id' | 'at'>) {
  const full: ActivityEntry = { ...entry, id: newId(), at: Date.now() };
  activity.update((list) => [full, ...list].slice(0, MAX));
}

export function clearActivity() {
  activity.set([]);
}

import { writable } from 'svelte/store';
import type { Source } from '$lib/types';

export type ActivityAction = 'install' | 'update' | 'uninstall' | 'update-all' | 'setup';

export interface ActivityEntry {
  id: string;
  action: ActivityAction;
  name: string;
  source?: Source;
  ok: boolean;
  at: number;
}

const KEY = 'acy-activity';
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

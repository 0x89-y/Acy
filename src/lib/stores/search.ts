import { writable, get } from 'svelte/store';
import * as api from '$lib/api';
import type { Package, SearchHit, Source } from '$lib/types';
import { enabledSources } from './managers';

/** Text in the sidebar search box. */
export const searchQuery = writable('');

export type SourceSearchState = 'searching' | 'done' | 'error';

export interface ManagerSearch {
  /** The query these results are for. */
  query: string;
  status: Partial<Record<Source, SourceSearchState>>;
  /** Merged across managers: one hit per app name, variants in priority order. */
  hits: SearchHit[];
}

export const managerSearch = writable<ManagerSearch>({ query: '', status: {}, hits: [] });

const PRIORITY: Record<Source, number> = { winget: 0, scoop: 1, choco: 2, msstore: 3 };

// Same rule as the backend's `sources::merge`, so a hit from winget and one from
// scoop for the same app become one result.
function dedupKey(name: string): string {
  return name.toLowerCase().replace(/[^\p{L}\p{N}]/gu, '');
}

function mergeHits(perSource: Partial<Record<Source, SearchHit[]>>): SearchHit[] {
  const map = new Map<string, SearchHit>();
  for (const hits of Object.values(perSource)) {
    for (const hit of hits ?? []) {
      const k = dedupKey(hit.name);
      const cur = map.get(k);
      if (!cur) {
        map.set(k, { ...hit, variants: [...hit.variants] });
        continue;
      }
      cur.installed ||= hit.installed;
      cur.publisher ??= hit.publisher;
      cur.description ??= hit.description;
      const have = new Set(cur.variants.map((v: Package) => `${v.source}:${v.id}`));
      for (const v of hit.variants) if (!have.has(`${v.source}:${v.id}`)) cur.variants.push(v);
    }
  }
  const out = [...map.values()];
  for (const h of out) h.variants.sort((a, b) => PRIORITY[a.source] - PRIORITY[b.source]);
  return out.sort((a, b) => a.name.toLowerCase().localeCompare(b.name.toLowerCase()));
}

let current = '';
let perSource: Partial<Record<Source, SearchHit[]>> = {};
const inFlight = new Set<Source>();
const pending = new Map<Source, string>();

async function runSource(source: Source, query: string) {
  inFlight.add(source);
  try {
    const hits = await api.search(query, [source]);
    if (query === current) {
      perSource = { ...perSource, [source]: hits };
      managerSearch.update((s) => ({
        ...s,
        status: { ...s.status, [source]: 'done' },
        hits: mergeHits(perSource)
      }));
    }
  } catch {
    if (query === current) {
      managerSearch.update((s) => ({ ...s, status: { ...s.status, [source]: 'error' } }));
    }
  } finally {
    inFlight.delete(source);
    // A manager runs one search at a time (winget is slow and spawns a process);
    // the newest query typed meanwhile runs next, older ones are dropped.
    const next = pending.get(source);
    pending.delete(source);
    if (next !== undefined && next === current) void runSource(source, next);
  }
}

/** Searches every enabled package manager; results fill in as each one answers. */
export function searchManagers(query: string) {
  const q = query.trim();
  if (!q || q === current) return;
  current = q;
  perSource = {};
  const sources = get(enabledSources);
  managerSearch.set({
    query: q,
    status: Object.fromEntries(sources.map((s) => [s, 'searching'])),
    hits: []
  });
  for (const source of sources) {
    if (inFlight.has(source)) pending.set(source, q);
    else void runSource(source, q);
  }
}

const DEBOUNCE_MS = 650;
let timer: ReturnType<typeof setTimeout> | undefined;

/** Searches once typing pauses (or right away, for Enter). Short queries wait for Enter. */
export function scheduleManagerSearch(query: string, immediate = false) {
  clearTimeout(timer);
  const q = query.trim();
  if (!q) return;
  if (immediate) searchManagers(q);
  else if (q.length >= 3) timer = setTimeout(() => searchManagers(q), DEBOUNCE_MS);
}

const RECENT_KEY = 'acy-recent-searches';

export function loadRecentSearches(): string[] {
  try {
    const v = JSON.parse(localStorage.getItem(RECENT_KEY) ?? '[]');
    return Array.isArray(v) ? v.filter((x) => typeof x === 'string').slice(0, 6) : [];
  } catch {
    return [];
  }
}

export function recordRecentSearch(q: string) {
  const list = [q, ...loadRecentSearches().filter((r) => r.toLowerCase() !== q.toLowerCase())];
  try {
    localStorage.setItem(RECENT_KEY, JSON.stringify(list.slice(0, 6)));
  } catch {
    // Storage can be unavailable; recent searches are only a convenience.
  }
}

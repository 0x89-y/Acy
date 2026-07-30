import { get, writable } from 'svelte/store';
import * as api from '$lib/api';
import { getCurated, saveCurated } from '$lib/api';
import { settings } from '$lib/stores/settings';
import type { CuratedApp, CuratedFile, SearchHit, Source, Variant } from '$lib/types';

const UNCATEGORIZED = 'uncategorized';

export function curatedKey(source: Source, id: string): string {
  return `${source}:${id.trim().toLowerCase()}`;
}

export function curatedKeys(file: CuratedFile | null): Set<string> {
  const set = new Set<string>();
  if (!file) return set;
  for (const cat of file.categories) {
    for (const app of cat.apps) {
      set.add(curatedKey(app.source, app.id));
      for (const alt of app.alternates ?? []) set.add(curatedKey(alt.source, alt.id));
    }
  }
  return set;
}

export const curated = writable<CuratedFile | null>(null);

let started = false;

export async function loadCurated() {
  if (started) return;
  started = true;
  try {
    curated.set(await getCurated());
  } catch (e) {
    started = false;
    console.error('curated load failed', e);
  }
}

export async function reloadCurated() {
  try {
    curated.set(await getCurated());
    started = true;
  } catch (e) {
    console.error('curated reload failed', e);
  }
}

export type CatalogStatus = 'unknown' | 'ready' | 'missing' | 'downloading' | 'error' | 'off';

export interface CatalogState {
  status: CatalogStatus;
  version: number;
  appCount: number;
  message: string;
}

export const catalog = writable<CatalogState>({
  status: 'unknown',
  version: 0,
  appCount: 0,
  message: ''
});

export async function loadCatalogStatus(): Promise<void> {
  try {
    const st = await api.curatedCatalogStatus();
    catalog.update((c) => ({
      ...c,
      status: st.present ? 'ready' : get(settings).showCuratedApps ? 'missing' : 'off',
      version: st.version,
      appCount: st.appCount,
      message: ''
    }));
  } catch (e) {
    console.error('catalog status failed', e);
  }
}

export async function downloadCatalog(): Promise<boolean> {
  const before = get(catalog);
  catalog.set({ ...before, status: 'downloading', message: '' });
  try {
    const res = await api.updateCuratedCatalog(true);
    if (res.updated) await reloadCurated();
    await loadCatalogStatus();
    return true;
  } catch (e) {
    const message = typeof e === 'string' ? e : "Couldn't download the catalog.";
    const st = await api.curatedCatalogStatus().catch(() => null);
    catalog.set({
      status: st?.present ? 'ready' : 'error',
      version: st?.version ?? before.version,
      appCount: st?.appCount ?? before.appCount,
      message
    });
    return !!st?.present;
  }
}

export async function syncCatalog(): Promise<void> {
  const s = get(settings);
  if (!s.setupComplete) return;
  if (!s.showCuratedApps) {
    await loadCatalogStatus();
    return;
  }
  try {
    const st = await api.curatedCatalogStatus();
    if (st.custom) {
      catalog.set({ status: 'ready', version: st.version, appCount: st.appCount, message: '' });
      return;
    }
  } catch {
  }
  await downloadCatalog();
}

export type AddResult = 'added' | 'exists' | 'error';

export interface AddToCuratedInput {
  source: Source;
  id: string;
  name?: string | null;
  description?: string | null;
  homepage?: string | null;
  alternates?: Variant[];
}

export function searchHitToInput(hit: SearchHit): AddToCuratedInput {
  const [primary, ...rest] = hit.variants;
  return {
    source: primary.source,
    id: primary.id,
    name: hit.name,
    description: hit.description ?? primary.description ?? null,
    homepage: primary.homepage ?? null,
    alternates: rest.map((v) => ({ source: v.source, id: v.id }))
  };
}

export async function moveCuratedApp(
  source: Source,
  id: string,
  toCategoryId: string
): Promise<boolean> {
  try {
    const file = await getCurated();
    const idLower = id.trim().toLowerCase();
    let moved: CuratedApp | null = null;
    for (const cat of file.categories) {
      const idx = cat.apps.findIndex((a) => a.source === source && a.id.toLowerCase() === idLower);
      if (idx !== -1) {
        moved = cat.apps[idx];
        cat.apps.splice(idx, 1);
        break;
      }
    }
    const target = file.categories.find((c) => c.id === toCategoryId);
    if (!moved || !target) return false;
    target.apps.push(moved);
    await saveCurated(file);
    await reloadCurated();
    return true;
  } catch (e) {
    console.error('move curated app failed', e);
    return false;
  }
}

export async function addToCurated(input: AddToCuratedInput): Promise<AddResult> {
  try {
    const file = await getCurated();
    const keys = curatedKeys(file);
    const candidates = [{ source: input.source, id: input.id }, ...(input.alternates ?? [])];
    if (candidates.some((v) => keys.has(curatedKey(v.source, v.id)))) return 'exists';

    const app: CuratedApp = {
      id: input.id,
      source: input.source,
      name: input.name ?? null,
      description: input.description ?? null,
      homepage: input.homepage ?? null,
      icon: null,
      alternates: input.alternates ?? [],
      tags: [],
      donate: null,
      releaseNotes: null,
      custom: true
    };

    let cat = file.categories.find((c) => c.id === UNCATEGORIZED);
    if (!cat) {
      cat = { id: UNCATEGORIZED, title: 'Uncategorized', apps: [] };
      file.categories.push(cat);
    }
    cat.apps.push(app);

    await saveCurated(file);
    await reloadCurated();
    return 'added';
  } catch (e) {
    console.error('add to curated failed', e);
    return 'error';
  }
}

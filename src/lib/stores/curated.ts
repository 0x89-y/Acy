import { writable } from 'svelte/store';
import { getCurated, saveCurated } from '$lib/api';
import type { CuratedApp, CuratedFile, SearchHit, Source, Variant } from '$lib/types';

const UNCATEGORIZED = 'uncategorized';

/** Stable key for matching an app across catalogs: source + lowercased id. */
export function curatedKey(source: Source, id: string): string {
  return `${source}:${id.trim().toLowerCase()}`;
}

/** Every (source, id) already in the catalog - primaries and alternates. */
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

// Shared curated catalog, loaded once and reused (e.g. by the detail page to
// show hardcoded info without hitting the package manager).
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

/** Force a re-fetch of the shared catalog (e.g. after a remote catalog update). */
export async function reloadCurated() {
  try {
    curated.set(await getCurated());
    started = true;
  } catch (e) {
    console.error('curated reload failed', e);
  }
}

export type AddResult = 'added' | 'exists' | 'error';

/** Minimal input to add an app to the user's list. */
export interface AddToCuratedInput {
  source: Source;
  id: string;
  name?: string | null;
  description?: string | null;
  homepage?: string | null;
  alternates?: Variant[];
}

/** Build add-input from a package-manager search hit (first variant is primary,
 * the rest become alternate install options). */
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

/** Move a curated app to another category (by id). Returns false if the app or
 * target category isn't found. Persists and refreshes the shared store. */
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

/** Add an app to the user's curated list under an "Uncategorized" category
 * (created if needed). No-op if any of its variants is already in the catalog.
 * Persists and refreshes the shared store. */
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

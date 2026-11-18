import type { CuratedApp, CuratedCategory, CuratedFile, Source, Variant } from '$lib/types';

/** `source:id` (id lowercased) - how installed state and the queue match apps. */
export function variantKey(v: Variant): string {
  return `${v.source}:${v.id.toLowerCase()}`;
}

/** An app's install options on enabled managers, primary first, one per manager. */
export function curatedVariants(app: CuratedApp, managers: Record<Source, boolean>): Variant[] {
  const seen = new Set<Source>();
  const out: Variant[] = [];
  for (const v of [{ source: app.source, id: app.id }, ...app.alternates]) {
    if (managers[v.source] === false || seen.has(v.source)) continue;
    seen.add(v.source);
    out.push(v);
  }
  return out;
}

/**
 * Categories as Discover shows them: only apps installable with the enabled
 * managers, and only the user's own apps when curated apps are turned off.
 */
export function browseCategories(
  file: CuratedFile | null,
  managers: Record<Source, boolean>,
  showCurated: boolean
): CuratedCategory[] {
  return (file?.categories ?? [])
    .map((cat) => ({
      ...cat,
      apps: cat.apps.filter(
        (a) => (showCurated || a.custom) && curatedVariants(a, managers).length > 0
      )
    }))
    .filter((cat) => cat.apps.length > 0);
}

/** Every distinct app across categories (an app can sit in more than one). */
export function uniqueApps(categories: CuratedCategory[]): CuratedApp[] {
  const seen = new Set<string>();
  const out: CuratedApp[] = [];
  for (const c of categories)
    for (const a of c.apps) {
      const k = variantKey(a);
      if (seen.has(k)) continue;
      seen.add(k);
      out.push(a);
    }
  return out;
}

export interface ResolvedCollection {
  id: string;
  title: string;
  description: string | null;
  apps: CuratedApp[];
}

/** Collections with their apps looked up in the visible categories; empty ones dropped. */
export function resolveCollections(
  file: CuratedFile | null,
  categories: CuratedCategory[]
): ResolvedCollection[] {
  const byKey = new Map<string, CuratedApp>();
  for (const a of uniqueApps(categories)) {
    byKey.set(variantKey(a), a);
    for (const alt of a.alternates) if (!byKey.has(variantKey(alt))) byKey.set(variantKey(alt), a);
  }
  return (file?.collections ?? [])
    .map((c) => {
      const seen = new Set<CuratedApp>();
      const apps: CuratedApp[] = [];
      for (const ref of c.apps) {
        const app = byKey.get(variantKey(ref));
        if (app && !seen.has(app)) {
          seen.add(app);
          apps.push(app);
        }
      }
      return { id: c.id, title: c.title, description: c.description ?? null, apps };
    })
    .filter((c) => c.apps.length > 0);
}

import { writable } from 'svelte/store';
import type { Package, Source } from '$lib/types';

// Ignoring is version-specific: a package returns to the Updates list when a
// newer version appears instead of being hidden forever.
const KEY = 'acy-ignored-updates';

function load(): Set<string> {
  if (typeof localStorage === 'undefined') return new Set();
  try {
    const value = JSON.parse(localStorage.getItem(KEY) ?? '[]');
    return new Set(Array.isArray(value) ? value : []);
  } catch {
    return new Set();
  }
}

export function ignoredUpdateKey(
  source: Source,
  id: string,
  availableVersion: string | null
): string {
  return `${source}:${id.toLowerCase()}:${availableVersion ?? ''}`;
}

export function packageIgnoredKey(pkg: Package): string {
  return ignoredUpdateKey(pkg.source, pkg.id, pkg.availableVersion);
}

export const ignoredUpdateKeys = writable<Set<string>>(load());

ignoredUpdateKeys.subscribe((keys) => {
  if (typeof localStorage === 'undefined') return;
  try {
    localStorage.setItem(KEY, JSON.stringify([...keys]));
  } catch {
    // Best-effort preference storage.
  }
});

export function ignoreUpdate(pkg: Package) {
  ignoredUpdateKeys.update((keys) => new Set(keys).add(packageIgnoredKey(pkg)));
}

export function restoreUpdate(pkg: Package) {
  ignoredUpdateKeys.update((keys) => {
    const next = new Set(keys);
    next.delete(packageIgnoredKey(pkg));
    return next;
  });
}

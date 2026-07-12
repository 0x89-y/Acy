import { writable } from 'svelte/store';
import { goto } from '$app/navigation';

/** A tag to activate on the Discover browse view, set by clicking a tag chip on
 * a card or the app page. The Discover page consumes and clears it. */
export const pendingTag = writable<string | null>(null);

/** Which mode the home panel shows: curated Discover browse, or the Library
 * (Installed / Updates). Lives in a store so nav shortcuts and the /installed
 * redirect can drive it even when the home route is already mounted. */
export const browseView = writable<'discover' | 'library'>('discover');

/** Jump to Discover and filter its browse view by a single tag. */
export function filterByTag(tag: string) {
  pendingTag.set(tag);
  goto('/');
}

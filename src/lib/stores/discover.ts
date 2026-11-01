import { writable } from 'svelte/store';
import { goto } from '$app/navigation';
import type { LibSelection } from '$lib/installedGroups';

export const pendingTag = writable<string | null>(null);

export const browseView = writable<'discover' | 'library'>('discover');

export const librarySection = writable<LibSelection>('all');

export function filterByTag(tag: string) {
  pendingTag.set(tag);
  goto('/');
}

export function openLibrary(section: LibSelection) {
  browseView.set('library');
  librarySection.set(section);
  goto('/');
}

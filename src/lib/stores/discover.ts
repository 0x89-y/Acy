import { writable } from 'svelte/store';
import { goto } from '$app/navigation';

export const pendingTag = writable<string | null>(null);

export const browseView = writable<'discover' | 'library'>('discover');

export function filterByTag(tag: string) {
  pendingTag.set(tag);
  goto('/');
}

import { writable } from 'svelte/store';
import { getCurated } from '$lib/api';
import type { CuratedFile } from '$lib/types';

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

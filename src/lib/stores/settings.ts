import { writable } from 'svelte/store';
import type { Source } from '$lib/types';

export type ThemeMode = 'light' | 'dark' | 'system';

export type AccentName = 'purple' | 'blue' | 'green' | 'pink' | 'orange' | 'teal';

export const ACCENTS: { name: AccentName; label: string; color: string }[] = [
  { name: 'purple', label: 'Purple', color: '#7c3aed' },
  { name: 'blue', label: 'Blue', color: '#2563eb' },
  { name: 'green', label: 'Green', color: '#059669' },
  { name: 'pink', label: 'Pink', color: '#db2777' },
  { name: 'orange', label: 'Orange', color: '#ea580c' },
  { name: 'teal', label: 'Teal', color: '#0d9488' }
];

export interface Settings {
  themeMode: ThemeMode;
  accent: AccentName;
  managers: Record<Source, boolean>;
  showOutput: boolean;
  downloadIcons: boolean;
}

const KEY = 'acy-settings';

const DEFAULTS: Settings = {
  themeMode: 'system',
  accent: 'purple',
  managers: { winget: true, scoop: true, choco: true },
  showOutput: false,
  downloadIcons: false
};

function load(): Settings {
  if (typeof localStorage !== 'undefined') {
    try {
      const raw = localStorage.getItem(KEY);
      if (raw) {
        const parsed = JSON.parse(raw);
        return {
          ...DEFAULTS,
          ...parsed,
          managers: { ...DEFAULTS.managers, ...(parsed.managers ?? {}) }
        };
      }
    } catch {
    }
  }
  return DEFAULTS;
}

export const settings = writable<Settings>(load());

settings.subscribe((value) => {
  if (typeof localStorage !== 'undefined') {
    localStorage.setItem(KEY, JSON.stringify(value));
  }
});

export function setThemeMode(mode: ThemeMode) {
  settings.update((s) => ({ ...s, themeMode: mode }));
}

export function setAccent(accent: AccentName) {
  settings.update((s) => ({ ...s, accent }));
}

export function setManagerEnabled(source: Source, enabled: boolean) {
  settings.update((s) => ({ ...s, managers: { ...s.managers, [source]: enabled } }));
}

export function setShowOutput(value: boolean) {
  settings.update((s) => ({ ...s, showOutput: value }));
}

export function setDownloadIcons(value: boolean) {
  settings.update((s) => ({ ...s, downloadIcons: value }));
}

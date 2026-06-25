import { writable, derived } from 'svelte/store';
import { settings } from './settings';

const prefersDark = () =>
  typeof matchMedia !== 'undefined' && matchMedia('(prefers-color-scheme: dark)').matches;

const systemDark = writable(prefersDark());

if (typeof matchMedia !== 'undefined') {
  matchMedia('(prefers-color-scheme: dark)').addEventListener('change', (e) =>
    systemDark.set(e.matches)
  );
}

/** The effective theme actually applied, resolving "system" to light/dark. */
export const appliedTheme = derived([settings, systemDark], ([$settings, $systemDark]) => {
  if ($settings.themeMode === 'system') return $systemDark ? 'dark' : 'light';
  return $settings.themeMode;
});

appliedTheme.subscribe((value) => {
  if (typeof document !== 'undefined') {
    document.documentElement.setAttribute('data-theme', value);
  }
});

settings.subscribe((s) => {
  if (typeof document !== 'undefined') {
    document.documentElement.setAttribute('data-accent', s.accent ?? 'purple');
  }
});

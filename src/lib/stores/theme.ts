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
  if (typeof document === 'undefined') return;
  const root = document.documentElement;
  const accent = s.accent ?? 'purple';
  if (accent === 'custom' && s.customAccent) {
    root.setAttribute('data-accent', 'custom');
    root.style.setProperty('--accent', s.customAccent);
  } else {
    root.setAttribute('data-accent', accent);
    root.style.removeProperty('--accent');
  }
});

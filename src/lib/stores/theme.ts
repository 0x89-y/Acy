import { writable, derived } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
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

type Rgb = [number, number, number];

function parseHex(hex: string): Rgb | null {
  const m = /^#?([0-9a-f]{6})$/i.exec(hex.trim());
  if (!m) return null;
  const n = parseInt(m[1], 16);
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}

function toHex([r, g, b]: Rgb): string {
  return '#' + [r, g, b].map((c) => Math.round(c).toString(16).padStart(2, '0')).join('');
}

function luminance([r, g, b]: Rgb): number {
  const f = (c: number) => {
    const s = c / 255;
    return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * f(r) + 0.7152 * f(g) + 0.0722 * f(b);
}

function contrast(a: Rgb, b: Rgb): number {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (hi + 0.05) / (lo + 0.05);
}

function mix(a: Rgb, b: Rgb, t: number): Rgb {
  return [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t];
}

const WHITE: Rgb = [255, 255, 255];
const INK: Rgb = [24, 24, 27];
// The surface-hover colours: the hardest background accent text sits on.
const HOVER_BG: Record<'light' | 'dark', Rgb> = {
  light: [240, 240, 242],
  dark: [46, 46, 52]
};

/** Tokens for a user-picked accent that keep WCAG AA (4.5:1) wherever they're used. */
export function customAccentTokens(hex: string, theme: 'light' | 'dark') {
  const base = parseHex(hex);
  if (!base) return null;
  // Text: nudge toward white (dark theme) or black (light theme) until it reads.
  const toward = theme === 'dark' ? WHITE : ([0, 0, 0] as Rgb);
  let text = base;
  for (let t = 0.05; contrast(text, HOVER_BG[theme]) < 4.5 && t <= 1; t += 0.05) {
    text = mix(base, toward, t);
  }
  // Fill: keep the picked colour; pick whichever label colour reads better.
  const onFill = contrast(WHITE, base) >= contrast(INK, base) ? WHITE : INK;
  // Hover darkens a fill that carries white text and lightens one that carries ink.
  const hover = mix(base, onFill === WHITE ? ([0, 0, 0] as Rgb) : WHITE, 0.15);
  return {
    accent: toHex(text),
    fill: toHex(base),
    hover: toHex(hover),
    contrast: toHex(onFill)
  };
}

const CUSTOM_PROPS = ['--accent', '--accent-fill', '--accent-hover', '--accent-contrast'];

// The shell (title bar + sidebar) colour without a backdrop; mirrors --bg in app.css.
const SHELL: Record<'light' | 'dark', string> = { light: '#f4f4f5', dark: '#161619' };

let lastStyle = '';

// Native title bar: a backdrop (Mica/Mica Alt/Acrylic) on Windows 11, with the
// shell transparent so it shows through; otherwise a caption painted the shell colour.
derived([settings, appliedTheme], (v) => v).subscribe(([s, theme]) => {
  if (typeof document === 'undefined' || !('__TAURI_INTERNALS__' in window)) return;
  const dark = theme === 'dark';
  const args = {
    theme: s.themeMode === 'system' ? null : s.themeMode,
    dark,
    backdrop: s.backdrop,
    caption: SHELL[dark ? 'dark' : 'light']
  };
  const key = JSON.stringify(args);
  if (key === lastStyle) return;
  lastStyle = key;
  invoke<boolean>('set_window_style', args)
    // The value (mica / mica-alt / acrylic) lets CSS tune each backdrop.
    .then((on) => {
      if (on) document.documentElement.setAttribute('data-backdrop', s.backdrop);
      else document.documentElement.removeAttribute('data-backdrop');
    })
    .catch(() => document.documentElement.removeAttribute('data-backdrop'));
});

derived([settings, appliedTheme], (v) => v).subscribe(([s, theme]) => {
  if (typeof document === 'undefined') return;
  const root = document.documentElement;
  const accent = s.accent ?? 'purple';
  const tokens =
    accent === 'custom' && s.customAccent ? customAccentTokens(s.customAccent, theme) : null;
  if (tokens) {
    root.setAttribute('data-accent', 'custom');
    root.style.setProperty('--accent', tokens.accent);
    root.style.setProperty('--accent-fill', tokens.fill);
    root.style.setProperty('--accent-hover', tokens.hover);
    root.style.setProperty('--accent-contrast', tokens.contrast);
  } else {
    root.setAttribute('data-accent', accent === 'custom' ? 'purple' : accent);
    for (const p of CUSTOM_PROPS) root.style.removeProperty(p);
  }
});

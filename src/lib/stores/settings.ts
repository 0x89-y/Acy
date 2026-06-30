import { writable } from 'svelte/store';
import type { Source } from '$lib/types';

export type ThemeMode = 'light' | 'dark' | 'system';

export type AccentName = 'purple' | 'blue' | 'green' | 'pink' | 'orange' | 'teal';

export type SettingsTab = 'general' | 'sources' | 'updates' | 'about';

export type ViewMode = 'grid' | 'list';

/** Accent presets, with a representative swatch color for the picker. */
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
  /** Per-source enable flag. A manager is only used when enabled AND available. */
  managers: Record<Source, boolean>;
  /** Preferred source for apps offered by several; null = decide each time. */
  preferredSource: Source | null;
  /** Expand the raw command output by default in the install drawer. */
  showOutput: boolean;
  /** Fetch + cache app icons from the web (off by default). */
  downloadIcons: boolean;
  /** Whether the first-run setup screen has been completed. */
  setupComplete: boolean;
  /** Closing the window hides to the tray instead of quitting (off by default). */
  closeToTray: boolean;
  /** Offer "minimize to tray?" when closing while closeToTray is off. */
  askCloseToTray: boolean;
  /** Show a desktop notification when background checks find new updates. */
  notifyUpdates: boolean;
  /** Notify when a long operation finishes while the window is in the background. */
  notifyOperations: boolean;
  /** Check installed apps / updates automatically when Acy starts. */
  refreshOnStartup: boolean;
  /** Check for app (Acy) updates on startup and periodically. Off by default. */
  autoCheckUpdates: boolean;
  /** Sticky UI preferences. */
  installedSort: 'name' | 'source';
  installedGroup: boolean;
  settingsTab: SettingsTab;
  discoverView: ViewMode;
  installedView: ViewMode;
}

const KEY = 'acy-settings';

const DEFAULTS: Settings = {
  themeMode: 'system',
  accent: 'purple',
  managers: { winget: true, scoop: true, choco: true, msstore: true, local: true },
  preferredSource: null,
  showOutput: false,
  downloadIcons: false,
  setupComplete: false,
  closeToTray: false,
  askCloseToTray: true,
  notifyUpdates: false,
  notifyOperations: false,
  refreshOnStartup: true,
  autoCheckUpdates: false,
  installedSort: 'name',
  installedGroup: false,
  settingsTab: 'general',
  discoverView: 'grid',
  installedView: 'list'
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
      // fall through to defaults
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

export function setPreferredSource(source: Source | null) {
  settings.update((s) => ({ ...s, preferredSource: source }));
}

export function setShowOutput(value: boolean) {
  settings.update((s) => ({ ...s, showOutput: value }));
}

export function setDownloadIcons(value: boolean) {
  settings.update((s) => ({ ...s, downloadIcons: value }));
}

export function setCloseToTray(value: boolean) {
  settings.update((s) => ({ ...s, closeToTray: value }));
}

export function setAskCloseToTray(value: boolean) {
  settings.update((s) => ({ ...s, askCloseToTray: value }));
}

export function setNotifyUpdates(value: boolean) {
  settings.update((s) => ({ ...s, notifyUpdates: value }));
}

export function setNotifyOperations(value: boolean) {
  settings.update((s) => ({ ...s, notifyOperations: value }));
}

export function setRefreshOnStartup(value: boolean) {
  settings.update((s) => ({ ...s, refreshOnStartup: value }));
}

export function setAutoCheckUpdates(value: boolean) {
  settings.update((s) => ({ ...s, autoCheckUpdates: value }));
}

export function setInstalledSort(value: Settings['installedSort']) {
  settings.update((s) => ({ ...s, installedSort: value }));
}

export function setInstalledGroup(value: boolean) {
  settings.update((s) => ({ ...s, installedGroup: value }));
}

export function setSettingsTab(value: SettingsTab) {
  settings.update((s) => ({ ...s, settingsTab: value }));
}

export function setDiscoverView(value: ViewMode) {
  settings.update((s) => ({ ...s, discoverView: value }));
}

export function setInstalledView(value: ViewMode) {
  settings.update((s) => ({ ...s, installedView: value }));
}

export function completeSetup() {
  settings.update((s) => ({ ...s, setupComplete: true }));
}

export function restartSetup() {
  settings.update((s) => ({ ...s, setupComplete: false }));
}

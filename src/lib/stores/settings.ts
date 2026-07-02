import { writable } from 'svelte/store';
import type { Source } from '$lib/types';

export type ThemeMode = 'light' | 'dark' | 'system';

export type AccentName = 'purple' | 'blue' | 'green' | 'pink' | 'orange' | 'teal';

export type SettingsTab = 'general' | 'sources' | 'updates' | 'about';

export type ViewMode = 'grid' | 'list';

export type InstalledShow = 'all' | 'hide-system' | 'managed';

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
  preferredSource: Source | null;
  showOutput: boolean;
  downloadIcons: boolean;
  setupComplete: boolean;
  closeToTray: boolean;
  askCloseToTray: boolean;
  notifyUpdates: boolean;
  notifyOperations: boolean;
  refreshOnStartup: boolean;
  autoCheckUpdates: boolean;
  warnChocoAdmin: boolean;
  installedSort: 'name' | 'source';
  installedGroup: boolean;
  installedShow: InstalledShow;
  collapsedGroups: string[];
  hiddenApps: string[];
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
  warnChocoAdmin: true,
  installedSort: 'name',
  installedGroup: true,
  installedShow: 'all',
  collapsedGroups: ['games', 'drivers', 'fonts', 'ms-system', 'other'],
  hiddenApps: [],
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

export function setWarnChocoAdmin(value: boolean) {
  settings.update((s) => ({ ...s, warnChocoAdmin: value }));
}

export function setInstalledSort(value: Settings['installedSort']) {
  settings.update((s) => ({ ...s, installedSort: value }));
}

export function setInstalledGroup(value: boolean) {
  settings.update((s) => ({ ...s, installedGroup: value }));
}

export function setInstalledShow(value: InstalledShow) {
  settings.update((s) => ({ ...s, installedShow: value }));
}

export function setGroupCollapsed(key: string, collapsed: boolean) {
  settings.update((s) => {
    const set = new Set(s.collapsedGroups);
    if (collapsed) set.add(key);
    else set.delete(key);
    return { ...s, collapsedGroups: [...set] };
  });
}

export function hideApp(key: string) {
  settings.update((s) =>
    s.hiddenApps.includes(key) ? s : { ...s, hiddenApps: [...s.hiddenApps, key] }
  );
}

export function unhideApp(key: string) {
  settings.update((s) => ({ ...s, hiddenApps: s.hiddenApps.filter((k) => k !== key) }));
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

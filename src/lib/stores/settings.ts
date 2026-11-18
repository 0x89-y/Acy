import { writable } from 'svelte/store';
import type { Source } from '$lib/types';

export type ThemeMode = 'light' | 'dark' | 'system';

export type AccentName =
  | 'purple'
  | 'blue'
  | 'green'
  | 'pink'
  | 'orange'
  | 'teal'
  | 'aurora'
  | 'custom';

export type SettingsTab = 'general' | 'appearance' | 'sources' | 'updates' | 'shortcuts' | 'about';

export type Backdrop = 'off' | 'mica' | 'mica-alt' | 'acrylic';

export const BACKDROPS: { value: Backdrop; label: string }[] = [
  { value: 'off', label: 'Off' },
  { value: 'mica', label: 'Mica' },
  { value: 'mica-alt', label: 'Mica Alt' },
  { value: 'acrylic', label: 'Acrylic' }
];

export const SETTINGS_TABS: { id: SettingsTab; label: string }[] = [
  { id: 'general', label: 'General' },
  { id: 'appearance', label: 'Appearance' },
  { id: 'sources', label: 'Sources' },
  { id: 'updates', label: 'Updates' },
  { id: 'shortcuts', label: 'Shortcuts' },
  { id: 'about', label: 'About' }
];

export type ViewMode = 'grid' | 'list';

export const ACCENTS: { name: AccentName; label: string; color: string }[] = [
  { name: 'purple', label: 'Purple', color: '#7c3aed' },
  { name: 'blue', label: 'Blue', color: '#2563eb' },
  { name: 'green', label: 'Green', color: '#047857' },
  { name: 'pink', label: 'Pink', color: '#be185d' },
  { name: 'orange', label: 'Orange', color: '#c2410c' },
  { name: 'teal', label: 'Teal', color: '#0f766e' },
  { name: 'aurora', label: 'Aurora', color: '#9540b8' }
];

export interface Settings {
  themeMode: ThemeMode;
  accent: AccentName;
  customAccent: string;
  managers: Record<Source, boolean>;
  preferredSource: Source | null;
  /** Base folder for winget installs (each app gets a subfolder); '' = installer default. */
  installLocation: string;
  showOutput: boolean;
  downloadIcons: boolean;
  steamGridKey: string;
  setupComplete: boolean;
  closeToTray: boolean;
  askCloseToTray: boolean;
  notifyUpdates: boolean;
  notifyOperations: boolean;
  refreshOnStartup: boolean;
  autoCheckUpdates: boolean;
  warnChocoAdmin: boolean;
  showCuratedApps: boolean;
  hiddenApps: string[];
  settingsTab: SettingsTab;
  discoverView: ViewMode;
  /** Window backdrop behind the title bar + sidebar (Windows 11 only). */
  backdrop: Backdrop;
}

const KEY = 'acy-settings';

const DEFAULTS: Settings = {
  themeMode: 'system',
  accent: 'purple',
  customAccent: '#7c3aed',
  managers: { winget: true, scoop: true, choco: true, msstore: true },
  preferredSource: null,
  installLocation: '',
  showOutput: false,
  downloadIcons: false,
  steamGridKey: '',
  setupComplete: false,
  closeToTray: false,
  askCloseToTray: true,
  notifyUpdates: false,
  notifyOperations: false,
  refreshOnStartup: true,
  autoCheckUpdates: false,
  warnChocoAdmin: true,
  showCuratedApps: true,
  hiddenApps: [],
  settingsTab: 'general',
  discoverView: 'grid',
  backdrop: 'mica'
};

function load(): Settings {
  if (typeof localStorage !== 'undefined') {
    try {
      const raw = localStorage.getItem(KEY);
      if (raw) {
        const parsed = JSON.parse(raw);
        if (parsed.showCuratedApps === undefined && typeof parsed.onlyCustomApps === 'boolean') {
          parsed.showCuratedApps = !parsed.onlyCustomApps;
        }
        delete parsed.onlyCustomApps;
        // The `local` source was removed.
        delete parsed.managers?.local;
        if (parsed.preferredSource === 'local') parsed.preferredSource = null;
        // The Mica on/off switch became a backdrop choice.
        if (parsed.backdrop === undefined && parsed.mica === false) parsed.backdrop = 'off';
        delete parsed.mica;
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

export function resetSettings() {
  settings.update((s) => ({ ...DEFAULTS, setupComplete: s.setupComplete }));
}

export function setThemeMode(mode: ThemeMode) {
  settings.update((s) => ({ ...s, themeMode: mode }));
}

export function setAccent(accent: AccentName) {
  settings.update((s) => ({ ...s, accent }));
}

export function setCustomAccent(hex: string) {
  settings.update((s) => ({ ...s, accent: 'custom', customAccent: hex }));
}

export function setManagerEnabled(source: Source, enabled: boolean) {
  settings.update((s) => ({ ...s, managers: { ...s.managers, [source]: enabled } }));
}

export function setPreferredSource(source: Source | null) {
  settings.update((s) => ({ ...s, preferredSource: source }));
}

export function setInstallLocation(value: string) {
  settings.update((s) => ({ ...s, installLocation: value.trim() }));
}

export function setShowOutput(value: boolean) {
  settings.update((s) => ({ ...s, showOutput: value }));
}

export function setDownloadIcons(value: boolean) {
  settings.update((s) => ({ ...s, downloadIcons: value }));
}

export function setSteamGridKey(value: string) {
  settings.update((s) => ({ ...s, steamGridKey: value.trim() }));
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

export function setShowCuratedApps(value: boolean) {
  settings.update((s) => ({ ...s, showCuratedApps: value }));
}

export function hideApp(key: string) {
  settings.update((s) =>
    s.hiddenApps.includes(key) ? s : { ...s, hiddenApps: [...s.hiddenApps, key] }
  );
}

export function unhideApp(key: string) {
  settings.update((s) => ({ ...s, hiddenApps: s.hiddenApps.filter((k) => k !== key) }));
}

export function setBackdrop(value: Backdrop) {
  settings.update((s) => ({ ...s, backdrop: value }));
}

export function setSettingsTab(value: SettingsTab) {
  settings.update((s) => ({ ...s, settingsTab: value }));
}

export function setDiscoverView(value: ViewMode) {
  settings.update((s) => ({ ...s, discoverView: value }));
}

export function completeSetup() {
  settings.update((s) => ({ ...s, setupComplete: true }));
}

export function restartSetup() {
  settings.update((s) => ({ ...s, setupComplete: false }));
}

// Acy's own release notes, shown in Settings. Newest first. Keep entries short
// and user-facing; bump the top version to match package.json on release.

export interface ChangelogEntry {
  version: string;
  /** ISO date (YYYY-MM-DD). */
  date: string;
  changes: string[];
}

export const CHANGELOG: ChangelogEntry[] = [
  {
    version: '0.5.0',
    date: '2026-06-28',
    changes: [
      'Set up a machine fast: multi-select install on Discover and multi-select uninstall on Installed, in one click.',
      'Right-click menus on app cards and installed rows (install/uninstall, open details, copy id, open homepage).',
      'Copy an install command or package id from an app, and copy command output from any toast.',
      'Search upgrades: a clear button and Esc, recent searches, highlighted matches, and arrow-key navigation of results.',
      'Keyboard shortcuts: Ctrl+1/2/3 switch pages and Esc goes back.',
      'Uninstalling now asks to confirm, and Acy remembers its window size and position.',
      'Polish: loading skeletons, a Clear-all for toasts, and Installed sort/grouping and the Settings tab now stick.',
      'More curated apps.'
    ]
  },
  {
    version: '0.4.2',
    date: '2026-06-27',
    changes: [
      'Settings are reorganized into tabs (General, Sources, Updates, About) with a sidebar, instead of one long scroll.'
    ]
  },
  {
    version: '0.4.1',
    date: '2026-06-27',
    changes: [
      'Update checks now ask before downloading, instead of installing automatically.',
      'Clearer update status: "up to date" is green and an available version is highlighted.',
      'New opt-in setting to check for updates on startup and periodically in the background (off by default).'
    ]
  },
  {
    version: '0.4.0',
    date: '2026-06-27',
    changes: [
      'Acy can update itself — Settings → Software updates → Check for updates.',
      'Install from a local or network .exe/.msi: add "local" as a per-app source (toggleable, and selectable as your preferred source).',
      'Scoop buckets: add buckets like extras right from Settings, so Scoop apps that need them install cleanly.',
      'Failed installs and updates can be retried from the toast, with a hint when administrator rights are needed.',
      'The Installed page shows when it was last checked, and a setting lets you skip the update check on startup.',
      'Tidier curated catalog editor: collapsible categories/apps and a single unified sources list per app.'
    ]
  },
  {
    version: '0.3.0',
    date: '2026-06-26',
    changes: [
      'Search now filters your curated apps as you type and only queries the package managers when you press Enter or Search.',
      'App detail pages show every manager an app is available from, with the multi-source install button.',
      'Many curated apps now offer several package managers, and the Scoop/Chocolatey Picks groups were folded into the topical categories.',
      'The curated catalog keeps apps you add while refreshing the built-in list on update; the editor tags each app as built-in or custom.',
      'Fixed the window close button failing when the tray was enabled.'
    ]
  },
  {
    version: '0.2.0',
    date: '2026-06-25',
    changes: [
      'Microsoft Store apps: a new msstore source for searching and installing Store-only apps.',
      'Curated apps can offer several package managers at once — install from any via a split button, with an optional preferred source.',
      'Curated catalog editor in Settings — edit the Discover home page without touching JSON.',
      'System tray with an optional "close to tray" mode that keeps update checks running in the background.',
      'Desktop notifications (opt-in) when new updates are found, plus an update count in the tray tooltip.',
      'Background update checks refresh the nav badge periodically while the app is open.',
      'Redesigned Settings: a centered, left-aligned layout with toggle switches in place of checkboxes.'
    ]
  },
  {
    version: '0.1.0',
    date: '2026-06-25',
    changes: [
      'First-run setup screen: pick theme, accent, package managers, and icon caching.',
      'Settings: centered layout, GitHub link, MIT license, changelog, and activity log.',
      'Installed: update all managers at once, plus filter, sort, and group-by-source.',
      'Discover: focus search with Ctrl+K or /, and clearer empty states.',
      'Reduced-motion support that follows the OS setting.'
    ]
  }
];

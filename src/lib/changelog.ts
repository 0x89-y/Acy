
export interface ChangelogEntry {
  version: string;
  date: string;
  changes: string[];
}

export const CHANGELOG: ChangelogEntry[] = [
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

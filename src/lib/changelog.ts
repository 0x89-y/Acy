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
    version: '0.8.0',
    date: '2026-07-12',
    changes: [
      'Discover and Installed are now one screen with a Discover / Library switch. Discover browses the catalog by category; Library lists your installed apps grouped by source (winget, Scoop, Chocolatey, Microsoft Store, Games, Windows components, Other apps), plus Updates.',
      'The home screen fills the whole window: the category/source list sits on the left and its apps on the right, in grid or list view.',
      'Installed apps now show their catalog description and tags, and grid/list view is shared with Discover, so both look the same.',
      'A package manager that still needs setting up appears in the Library list and opens Settings → Sources; the Library tab shows a dot when an update is available or a source needs attention.',
      'Reworked the look to match the shared design: flat surfaces (no shadows), brighter accent colours with white text, squircle app icons, and a slimmer title bar.',
      'Settings is now one clean panel with a back button, and the top bar no longer carries a Discover link — the logo takes you home.',
      'Installed no longer has separate "Drivers" and "Fonts" groups; they sit under "Other apps".'
    ]
  },
  {
    version: '0.7.1',
    date: '2026-07-04',
    changes: [
      'Curated apps can now carry a "Release notes" link — it shows on the app page and on its update row in Installed, so you can see what changed before updating.',
      'Tags are clickable everywhere (cards and the app page): click one to filter Discover by it, and Discover search now matches tags too (e.g. "free").',
      'Added a custom accent colour picker (Settings → Appearance) that adapts to light and dark automatically.',
      'Added "Reset all settings" to restore theme, accent, managers, and preferences to defaults (your apps and curated list are untouched).',
      'Apps in the Uncategorized group can be filed into a category from their right-click menu.',
      'Discover catalog updates no longer apply automatically — like the app updater, "Check for catalog updates" tells you when a new version is available and you choose to apply it.',
      'Fixed the ▾ menu on the Install split button not opening.'
    ]
  },
  {
    version: '0.7.0',
    date: '2026-07-03',
    changes: [
      'Discover can now be updated without a new app version: Acy fetches a newer, signature-verified catalog from the web when you press "Check for catalog updates" (Settings → Sources → Curated catalog).',
      'Add any app you find in search to your Discover list with a "+" button, then edit or remove it right from the app\'s page — no need to open the catalog editor.',
      'The catalog editor gained a search box that matches app names, ids, tags, and category titles.',
      'Added an Aurora accent theme, and in light mode all the accent colours are now softer pastels with dark text.',
      'Combined the Settings action buttons (app icons, maintenance, catalog) and the changelog pager into single accent controls.',
      'Removed the light/dark toggle from the top bar — change the theme in Settings → Appearance instead.'
    ]
  },
  {
    version: '0.6.6',
    date: '2026-07-03',
    changes: [
      'Missing app icons can be re-downloaded from Settings → App icons, and icon fetching is gentler about rate limits — apps with no icon are remembered so they are not re-checked every time.',
      'Icons now show a loading placeholder while they load, with calmer fallback tiles.',
      'Reworked Settings: a new Appearance tab for theme, accent, and icons, and every section now uses the same clean bordered-list look.',
      "Scoop buckets have their own page, opened from a \"Manage buckets\" button on Scoop's row in Settings.",
      "The Installed list groups each manager's apps into a single bordered list."
    ]
  },
  {
    version: '0.6.5',
    date: '2026-07-02',
    changes: [
      'Refreshed the interface for a calmer, flatter look: muted accent colours, no glass blur on the top bar, squared-off badges and tags, and less explanatory clutter.',
      "Scoop installs that need a bucket you don't have (like extras for Firefox or VS Code) now offer to add it first instead of failing.",
      'Redesigned the Scoop buckets settings as a list you can add to and remove from, each with a short description.',
      'Added ALCOM, Bambu Studio, Ente Photos, Hytale, MusicBrainz Picard, TeamSpeak, VRChat Creator Companion, VRCX, and WowUp to Discover.',
      'Expanded filter tags across the catalog and added more verified support links.'
    ]
  },
  {
    version: '0.6.4',
    date: '2026-07-01',
    changes: [
      'Chocolatey actions now show a dismissible "may need administrator rights" warning instead of checking your admin status (which some antivirus flagged).',
      'Removed the "switch an app to another package manager" action, and softened how Acy invokes PowerShell, to avoid antivirus false positives.',
      "Acy's own entry can no longer show up in — or be removed from — the Installed list."
    ]
  },
  {
    version: '0.6.3',
    date: '2026-07-01',
    changes: [
      'Fixed the right-click "Switch to another manager" and "Uninstall" actions erroring instead of asking for confirmation.',
      'Chocolatey needs administrator rights: Acy now shows a clear "restart as administrator" message instead of failing when it isn\'t elevated.',
      'Added an "Admin" badge in the nav when Acy is running as administrator.'
    ]
  },
  {
    version: '0.6.2',
    date: '2026-07-01',
    changes: [
      'Installed apps now appear instantly (read straight from Windows), with winget filling in details in the background.',
      'With "refresh on startup" off, launches no longer run a slow winget scan — it only runs when you press Refresh, so installs and uninstalls no longer get stuck behind it.',
      'A slow or failed refresh no longer blanks the Installed list.',
      'Right-click a curated app to switch it between package managers (e.g. winget → choco).',
      'Games, drivers, and Windows system components now sort into their own collapsible groups more accurately; the list is grouped by default.',
      'Added a small indicator that shows when winget is scanning in the background.'
    ]
  },
  {
    version: '0.6.1',
    date: '2026-07-01',
    changes: [
      'Fixed winget hanging indefinitely: reads now time out with a clear error instead of freezing the app or leaving stuck processes.',
      'Fixed uninstalls and refreshes stalling when a source agreement or an administrator prompt was pending.',
      'Installed now sorts Steam/other games, drivers, and Windows system components into their own collapsible groups, and remembers what you collapse.',
      'New "Show" control (All / Hide system / Managed only) with a "managed · total" count.',
      'Right-click an app to hide it from the list; apps installed via more than one manager are flagged.',
      'Acy no longer lists itself twice.'
    ]
  },
  {
    version: '0.6.0',
    date: '2026-07-01',
    changes: [
      'Curated apps now have tags, and you can filter Discover by them.',
      'Many apps now show a "Donate" button linking to the developer\'s donation page.',
      'Discover list view shows app descriptions; cards no longer show the package id.',
      '"Copy command" and "Copy id" moved into the right-click menu.',
      'New Changelog and Activity pages, linked from Settings.',
      'Acy now appears in your Installed list, and Acy updates show alongside app updates.'
    ]
  },
  {
    version: '0.5.5',
    date: '2026-06-29',
    changes: [
      'Long categories now open a dedicated page with every app (the "Show more" link is now "View all"), keeping grid/list views and multi-select install.',
      'Back from an app\'s detail page returns to the category page you came from.',
      'More curated apps.'
    ]
  },
  {
    version: '0.5.4',
    date: '2026-06-28',
    changes: [
      'Fixed the installed-apps list failing to load on non-English Windows (e.g. German).',
      'The "Install WinGet PowerShell module" action now installs its NuGet prerequisite, so it works on a fresh machine and takes effect without restarting Acy.'
    ]
  },
  {
    version: '0.5.3',
    date: '2026-06-28',
    changes: [
      'Ignore a specific update so it stops prompting until a newer version is released.',
      'Tidier Discover: a steadier "Select apps" mode that no longer shifts the layout, and already-installed apps can no longer be selected.',
      'Grid and list view controls now sit consistently on the right.',
      'Simplified the search button and fixed Back navigation from an app\'s detail page.',
      'Smoother batch operations: clearer progress, confirmations, notifications, keyboard access, and history.'
    ]
  },
  {
    version: '0.5.2',
    date: '2026-06-28',
    changes: [
      'Launching Acy again now focuses the running window (even from the tray) instead of opening a second copy.',
      'Press ? to see all keyboard shortcuts.',
      'Multi-select install or uninstall now shows a single summary (e.g. "5 apps installed").',
      'New Maintenance actions in Settings: update winget sources, update Scoop, and clean up old Scoop versions.'
    ]
  },
  {
    version: '0.5.1',
    date: '2026-06-28',
    changes: [
      'Switch between grid and list view on Discover and Installed; your choice is remembered per page.',
      'Closing with "close to tray" off now offers to minimize to the tray instead, with a "don\'t ask again" option.',
      'An in-app banner now appears when an Acy update is found, so background update checks are visible.',
      'Restyled the multi-select checkboxes to match the rest of the UI.',
      'The default right-click menu no longer appears on empty areas (text fields keep copy/paste).'
    ]
  },
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

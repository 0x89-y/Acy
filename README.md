# Acy

Acy is a Windows desktop interface for WinGet, Scoop, Chocolatey, and the
Microsoft Store. It provides package search, installation, removal, and update
management. Local `.exe` and `.msi` installers can also be added to the curated
catalog.

## Install

Download the Windows installer from the
[latest release](https://github.com/0x89-y/Acy/releases/latest).

Acy detects the available package managers at startup. Missing managers can be
installed from the setup screen or enabled later in Settings. Microsoft Store
support uses WinGet.

## Sources

| Source | Search | Install | Uninstall | Updates |
| --- | --- | --- | --- | --- |
| WinGet | Yes | Yes | Yes | Yes |
| Scoop | Yes | Yes | Yes | Yes |
| Chocolatey | Yes | Yes | Yes | Yes |
| Microsoft Store | Yes | Yes | No | No |
| Local `.exe` / `.msi` | No | Yes | No | No |

Microsoft Store packages are accessed through WinGet's `msstore` source. Local
installers are files selected by the user and are not treated as managed
packages after installation.

## Appearance

Light, dark, or system theme, and an accent colour chosen from a set of presets
or a custom colour. Configured in Settings → Appearance.

## Development

Requirements:

- Node.js
- Rust
- [Tauri's Windows prerequisites](https://v2.tauri.app/start/prerequisites/)

Install dependencies and run the application:

```sh
npm install
npm run tauri dev
```

Other commands:

```sh
npm run check                                     # check Svelte and TypeScript
npm run build                                     # build the frontend
npm run tauri build                               # build Windows bundles
cargo test --manifest-path src-tauri/Cargo.toml   # run Rust tests
```

Tauri writes release bundles under
`src-tauri/target/release/bundle`. The repository's `build.bat` builds the NSIS
installer and expects an updater signing key at
`%USERPROFILE%\.acy\acy-updater.key`.

## Curated catalog

`curated.json` defines the categories and applications shown on the Discover
page. The catalog can also be edited from within Acy.

Each application has a primary package source. The optional `alternates` array
lists the same application in other sources. Package IDs are specific to their
source.

```json
{
  "version": 1,
  "categories": [
    {
      "id": "browsers",
      "title": "Browsers",
      "apps": [
        {
          "id": "Mozilla.Firefox",
          "source": "winget",
          "name": "Firefox",
          "alternates": [
            { "source": "scoop", "id": "firefox" },
            { "source": "choco", "id": "firefox" }
          ]
        }
      ]
    }
  ]
}
```

Optional application fields are `name`, `description`, `homepage`, `icon`,
`tags`, `donate`, and `releaseNotes`. Tags are used for filtering and search;
`donate` and `releaseNotes` are shown as links on the application's page. For
local installers, use `local` as the source. The package ID may contain the
installer path or be left empty so the file can be selected at install time.

Applications found through search can be added to the catalog from within Acy;
they are placed in an "Uncategorized" group until moved to a category. Entries
can be edited from an application's page or in the catalog editor.

The catalog is loaded as follows:

1. If `ACY_CURATED` contains a valid file path, that file replaces the catalog.
2. Otherwise the base catalog is the higher-`version` of the bundled copy and a
   downloaded catalog update (see below); the repository copy is used during
   development.
3. Custom entries from the per-user catalog are merged into that base catalog.
4. The embedded copy is used if no external base catalog can be read.

The per-user catalog is stored in Acy's application configuration directory.
Built-in entries are refreshed when Acy is updated; entries added by the user
are retained.

### Catalog updates

A signed catalog can be hosted so the Discover page can be updated without a new
application release. From Settings → Sources → Curated catalog, Acy checks for a
newer hosted catalog on request and applies it after confirmation; it does not
update automatically. The catalog's signature is verified before the downloaded
copy is trusted, and a newer built-in catalog is never downgraded.

## Permissions

Scoop installs packages in the current user's profile by default. WinGet and
Chocolatey operations may require elevation depending on the package and
installation scope. Windows displays a UAC prompt when elevation is requested.

## Project layout

```text
src/
  lib/                frontend components, stores, API wrappers, and styles
  routes/             Discover, Installed, Settings, catalog, and app pages
src-tauri/
  src/
    sources/          WinGet, Scoop, Chocolatey, Microsoft Store, and local backends
    commands.rs       Tauri command handlers
    curated.rs        catalog loading and merging
    model.rs          shared package and source types
    runner.rs         process execution and output streaming
    tray.rs           system tray integration
  tauri.conf.json     application and bundle configuration
curated.json          built-in catalog
```

## License

[MIT](LICENSE)

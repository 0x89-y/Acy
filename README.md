# Acy

A Windows app store built on top of the package managers you already have:
**winget**, **Scoop**, and **Chocolatey**. It gives them one interface with a
curated home page, unified search, install/uninstall/update management, and a
light/dark theme.

## What it does

- **Discover**: a curated home page grouped into categories you define in
  `curated.json`.
- **Search**: one search box across winget, Scoop, and Chocolatey, with results
  merged and de-duplicated. Toggle which managers are searched.
- **Installed + updates**: list everything installed, see available updates with
  a count badge, update a single app or update all from one manager.
- **App detail**: version, publisher, homepage, source, and install/uninstall/
  update actions per app.
- **Live output**: installs and updates stream their real output into a drawer
  instead of a spinner.
- **Manager setup**: detects which managers are present and offers a one-click
  bootstrap for any that are missing.

## Stack

- **Tauri 2** (Rust core) for a small, native Windows app.
- **SvelteKit + Svelte 5 + TypeScript** frontend in pure SPA mode.
- Plain CSS custom properties for theming. IBM Plex Sans / Mono, dot-grid identity.

The Rust core avoids scraping fragile CLI tables for reads when it can:

- **winget**: prefers the official `Microsoft.WinGet.Client` PowerShell module
  (clean JSON), and falls back to parsing the `winget` CLI when the module is
  not installed.
- **Scoop**: uses `scoop export` (JSON) for installed apps; parses `scoop search`
  and `scoop status` tables.
- **Chocolatey**: uses `-r` (`--limit-output`) for clean pipe-delimited rows.

Reads return structured data; writes (install/uninstall/upgrade) stream the CLI
so the UI shows live progress.

## Develop

Prerequisites: Node, Rust, and the Tauri prerequisites for Windows
(WebView2 ships with Windows 11).

```sh
npm install
npm run tauri dev      # run the app
npm run tauri build    # build an installer (MSI / NSIS) under src-tauri/target/release/bundle
```

Other useful commands:

```sh
npm run build                                      # build the frontend only
cargo test --manifest-path src-tauri/Cargo.toml    # run the parser unit tests
```

## Editing the curated list

`curated.json` (repo root) defines the home page. Each app names its `id` and
`source`; the optional `name` / `description` / `homepage` override what the
manager reports.

```json
{
  "version": 1,
  "categories": [
    {
      "id": "browsers",
      "title": "Browsers",
      "apps": [{ "id": "Mozilla.Firefox", "source": "winget", "name": "Firefox" }]
    }
  ]
}
```

Package ids are manager-specific: winget uses its `PackageIdentifier`
(`Mozilla.Firefox`), Scoop and Chocolatey use the package name (`neovim`,
`nodejs`).

At runtime the list is read from the first source found, in order:

1. the `ACY_CURATED` environment variable (a file path),
2. `%APPDATA%/Acy/curated.json` (a per-user override in a shipped build),
3. the repo-root `curated.json` during development (live edits, no rebuild),
4. the copy bundled with the app.

If none is found it uses the version embedded at build time.

## Notes on permissions

- **Scoop** installs to your user profile and needs no elevation.
- **winget** (machine scope) and **Chocolatey** usually need administrator
  rights. Installs that need elevation trigger a UAC prompt, and the error is
  shown in the live output if elevation is declined. Installing Chocolatey
  itself must be done from an elevated session.

## Layout

```
src-tauri/src/
  model.rs        normalized Package + Source types
  runner.rs       process capture + streaming helpers
  sources/        PackageSource trait + winget.rs / scoop.rs / choco.rs
  curated.rs      curated.json loading
  commands.rs     Tauri command surface
src/
  routes/         Discover (/), Installed, app detail (/app/[source]/[id])
  lib/            api wrappers, stores, components, theme
curated.json      the curated catalog
```

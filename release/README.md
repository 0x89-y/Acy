# Releasing Acy (self-update)

The desktop app checks `plugins.updater.endpoints` in `src-tauri/tauri.conf.json`
for a `latest.json` manifest. To publish an update, host the new installer and an
updated `latest.json` on that endpoint. `latest.json` here is a template — copy it,
fill the four fields, upload it.

## Steps for each release

1. **Bump the version** in all three files: `package.json`,
   `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, and add a
   `src/lib/changelog.ts` entry.
2. **Build + sign**: run `build.bat` (it sets the signing key and prompts for its
   password). Output lands in `src-tauri/target/release/bundle/nsis/`:
   - `Acy_<version>_x64-setup.exe`  ← the installer
   - `Acy_<version>_x64-setup.exe.sig`  ← its signature
3. **Fill `latest.json`** (copy of this template):
   - `version`   → the new version (must be higher than what users have).
   - `notes`     → short summary shown in the update prompt.
   - `pub_date`  → ISO 8601, e.g. `2026-06-26T12:00:00Z`.
   - `signature` → the **entire contents** of the `*-setup.exe.sig` file.
   - `url`       → the public URL where you upload the `-setup.exe`.
4. **Upload** the `-setup.exe` and the filled `latest.json` to your host
   (HTTPS), so the URLs match `tauri.conf.json` → `plugins.updater.endpoints`
   and the `url` field. Example layout:

   ```
   https://your-host/acy/
   ├── latest.json
   └── Acy_<version>_x64-setup.exe
   ```

That's it — the app's **Settings → Software updates → Check for updates** will now
offer the new version, verify its signature against the public key baked into the
app, install it, and relaunch.

## One-time setup

- Generate the signing keypair:
  `npx tauri signer generate -w "%USERPROFILE%\.acy\acy-updater.key"`
- Put the printed **public** key into `tauri.conf.json` → `plugins.updater.pubkey`.
- Point `plugins.updater.endpoints` at your `latest.json` URL.
- Keep the **private** key secret and backed up; losing it means existing installs
  can't accept future updates.

The Windows x64 platform key is exactly `windows-x86_64`. See the "Self-update"
section in the repo's `CLAUDE.md` for more.

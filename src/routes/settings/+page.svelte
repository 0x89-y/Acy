<script lang="ts">
  import { onMount } from 'svelte';
  import { getVersion } from '@tauri-apps/api/app';
  import { Sun, Moon, Monitor } from '@lucide/svelte';
  import {
    updaterPhase,
    updaterVersion,
    updaterError,
    checkForUpdate,
    installUpdate,
    backgroundCheck
  } from '$lib/stores/updater';
  import {
    settings,
    setThemeMode,
    setAccent,
    setManagerEnabled,
    setPreferredSource,
    setShowOutput,
    setDownloadIcons,
    setCloseToTray,
    setNotifyUpdates,
    setNotifyOperations,
    setRefreshOnStartup,
    setAutoCheckUpdates,
    setSettingsTab,
    restartSetup,
    ACCENTS,
    type ThemeMode,
    type SettingsTab
  } from '$lib/stores/settings';
  import { managers, loadManagers } from '$lib/stores/managers';
  import { clearIconCache } from '$lib/stores/icons';
  import { enqueue } from '$lib/stores/ops';
  import { confirmAction } from '$lib/stores/confirm';
  import { activity, type ActivityAction } from '$lib/stores/activity';
  import { CHANGELOG } from '$lib/changelog';
  import * as api from '$lib/api';
  import type { Source } from '$lib/types';

  const actionLabel: Record<ActivityAction, string> = {
    install: 'Installed',
    update: 'Updated',
    uninstall: 'Removed',
    'update-all': 'Updated all',
    setup: 'Set up'
  };

  function when(at: number): string {
    const d = new Date(at);
    return d.toLocaleString(undefined, {
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit'
    });
  }

  const modes: { value: ThemeMode; label: string; icon: typeof Sun }[] = [
    { value: 'light', label: 'Light', icon: Sun },
    { value: 'dark', label: 'Dark', icon: Moon },
    { value: 'system', label: 'System', icon: Monitor }
  ];

  const names: Record<Source, string> = {
    winget: 'winget',
    scoop: 'Scoop',
    choco: 'Chocolatey',
    msstore: 'Microsoft Store',
    local: 'Local file'
  };
  const allManagers: Source[] = ['winget', 'scoop', 'choco', 'msstore', 'local'];

  let busy = $state<Source | null>(null);
  let clearing = $state(false);
  let appVersion = $state('');
  const ACTIVITY_PREVIEW = 6;

  onMount(async () => {
    try {
      appVersion = await getVersion();
    } catch {
      appVersion = '';
    }
  });

  async function clearIcons() {
    clearing = true;
    await clearIconCache();
    clearing = false;
  }

  function statusOf(source: Source) {
    return $managers.find((m) => m.source === source);
  }

  async function install(source: Source) {
    busy = source;
    await enqueue(`Set up ${names[source]}`, (opId) => api.bootstrapManager(source, opId));
    busy = null;
    loadManagers(true);
  }

  let buckets = $state<string[] | null>(null);
  let knownBuckets = $state<string[]>([]);
  let bucketBusy = $state<string | null>(null);
  let scoopAvailable = $derived(statusOf('scoop')?.available ?? false);
  let wingetAvailable = $derived(statusOf('winget')?.available ?? false);

  let maintBusy = $state<string | null>(null);
  async function runMaint(label: string, k: string, fn: (opId: string) => Promise<number>) {
    maintBusy = k;
    await enqueue(label, fn);
    maintBusy = null;
  }

  async function loadBuckets() {
    try {
      const [b, k] = await Promise.all([api.scoopBuckets(), api.scoopKnownBuckets()]);
      buckets = b;
      knownBuckets = k;
    } catch {
      buckets = [];
    }
  }

  $effect(() => {
    if (scoopAvailable && buckets === null) loadBuckets();
  });

  const BUCKET_INFO: Record<string, string> = {
    main: 'Core command-line tools',
    extras: 'GUI apps — Firefox, VLC, Discord, VS Code…',
    versions: 'Alternate and older app versions',
    nirsoft: 'NirSoft utilities',
    games: 'Games and game tools',
    java: 'Java runtimes and JDKs',
    php: 'PHP versions',
    nonportable: 'Apps that need a full installer',
    sysinternals: 'Microsoft Sysinternals tools'
  };

  let bucketRows = $derived.by(() => {
    const added = buckets ?? [];
    const names = [...new Set([...added, ...knownBuckets])];
    return names
      .map((name) => ({
        name,
        added: added.includes(name),
        description: BUCKET_INFO[name] ?? (added.includes(name) ? 'Added bucket' : '')
      }))
      .sort((a, b) => {
        if (a.name === 'main') return -1;
        if (b.name === 'main') return 1;
        if (a.added !== b.added) return a.added ? -1 : 1;
        return a.name.localeCompare(b.name);
      });
  });

  async function addBucket(name: string) {
    bucketBusy = name;
    await enqueue(`Add Scoop bucket: ${name}`, (opId) => api.addScoopBucket(name, opId));
    bucketBusy = null;
    loadBuckets();
  }

  async function removeBucket(name: string) {
    const ok = await confirmAction({
      title: `Remove the "${name}" bucket?`,
      message:
        `Apps you already installed from "${name}" stay, but Scoop won't offer updates ` +
        `for them until you add it back.`,
      confirmLabel: 'Remove bucket',
      danger: true
    });
    if (!ok) return;
    bucketBusy = name;
    await enqueue(`Remove Scoop bucket: ${name}`, (opId) => api.removeScoopBucket(name, opId));
    bucketBusy = null;
    loadBuckets();
  }

  function onAutoCheckToggle(on: boolean) {
    setAutoCheckUpdates(on);
    if (on) backgroundCheck();
  }

  const tabs: { id: SettingsTab; label: string }[] = [
    { id: 'general', label: 'General' },
    { id: 'sources', label: 'Sources' },
    { id: 'updates', label: 'Updates' },
    { id: 'about', label: 'About' }
  ];
  let activeTab = $derived($settings.settingsTab);
</script>

<h1>Settings</h1>

<div class="settings-layout">
  <nav class="side" aria-label="Settings sections">
    {#each tabs as t (t.id)}
      <button class="side-link" class:active={activeTab === t.id} onclick={() => setSettingsTab(t.id)}>
        {t.label}
      </button>
    {/each}
  </nav>

  <div class="panes">
    {#if activeTab === 'general'}
      <section class="group">
        <h2>Appearance</h2>
        <div class="field">
          <span class="field-label">Theme</span>
          <div class="seg">
            {#each modes as m (m.value)}
              {@const Icon = m.icon}
              <button
                class="seg-btn"
                class:on={$settings.themeMode === m.value}
                onclick={() => setThemeMode(m.value)}
              >
                <Icon size={16} />
                {m.label}
              </button>
            {/each}
          </div>
        </div>
        <div class="field">
          <span class="field-label">Accent</span>
          <div class="accents">
            {#each ACCENTS as a (a.name)}
              <button
                class="swatch"
                class:on={$settings.accent === a.name}
                style="--sw:{a.color}"
                onclick={() => setAccent(a.name)}
                title={a.label}
                aria-label={a.label}
              ></button>
            {/each}
          </div>
        </div>
      </section>

      <section class="group">
        <h2>Operations</h2>
        <label class="toggle-row card">
          <span class="toggle-text">
            <span class="toggle-title">Show command output while installing</span>
          </span>
          <span class="switch">
            <input
              type="checkbox"
              checked={$settings.showOutput}
              onchange={(e) => setShowOutput(e.currentTarget.checked)}
            />
            <span class="slider"></span>
          </span>
        </label>
        <label class="toggle-row card">
          <span class="toggle-text">
            <span class="toggle-title">Check for updates on startup</span>
            <span class="toggle-sub muted">
              Refresh installed apps and available updates automatically when Acy starts. Turn off
              for a faster launch; you can still refresh manually on the Installed page.
            </span>
          </span>
          <span class="switch">
            <input
              type="checkbox"
              checked={$settings.refreshOnStartup}
              onchange={(e) => setRefreshOnStartup(e.currentTarget.checked)}
            />
            <span class="slider"></span>
          </span>
        </label>
        <label class="toggle-row card">
          <span class="toggle-text">
            <span class="toggle-title">Notify when long operations finish</span>
            <span class="toggle-sub muted">
              Shown only when an operation takes at least 15 seconds and Acy is in the background.
            </span>
          </span>
          <span class="switch">
            <input
              type="checkbox"
              checked={$settings.notifyOperations}
              onchange={(e) => setNotifyOperations(e.currentTarget.checked)}
            />
            <span class="slider"></span>
          </span>
        </label>
      </section>

      <section class="group">
        <h2>Tray &amp; notifications</h2>
        <label class="toggle-row card">
          <span class="toggle-text">
            <span class="toggle-title">Close to tray instead of quitting</span>
            <span class="toggle-sub muted">
              Acy keeps running in the system tray after you close the window and checks for updates
              in the background. Quit from the tray icon's menu.
            </span>
          </span>
          <span class="switch">
            <input
              type="checkbox"
              checked={$settings.closeToTray}
              onchange={(e) => setCloseToTray(e.currentTarget.checked)}
            />
            <span class="slider"></span>
          </span>
        </label>
        <label class="toggle-row card">
          <span class="toggle-text">
            <span class="toggle-title">Notify when new updates are found</span>
          </span>
          <span class="switch">
            <input
              type="checkbox"
              checked={$settings.notifyUpdates}
              onchange={(e) => setNotifyUpdates(e.currentTarget.checked)}
            />
            <span class="slider"></span>
          </span>
        </label>
      </section>

      <section class="group">
        <h2>First-run setup</h2>
        <p class="muted hint">Show the welcome setup screen again to reconfigure from scratch.</p>
        <button class="btn" onclick={restartSetup}>Run setup again</button>
      </section>
    {:else if activeTab === 'sources'}
      <section class="group source-group">
        <h2>Package managers</h2>
        <p class="muted hint">
          Turn managers on or off. A disabled manager is skipped in search, installed apps, and
          updates.
        </p>
        <div class="field pref">
          <span class="field-label">Preferred source</span>
          <select
            class="pref-select"
            value={$settings.preferredSource ?? ''}
            onchange={(e) => setPreferredSource((e.currentTarget.value || null) as Source | null)}
          >
            <option value="">No preference (choose each time)</option>
            {#each allManagers as s (s)}
              <option value={s}>{names[s]}</option>
            {/each}
          </select>
          <p class="muted hint">
            For apps offered by several managers, the Install button uses this one when available;
            you can still pick another from its menu.
          </p>
        </div>
        <div class="manager-grid">
          {#each allManagers as s (s)}
            {@const st = statusOf(s)}
            {@const isLocal = s === 'local'}
            <div class="row card manager-row">
              <div class="info">
                <span class="name">{names[s]}</span>
                <span
                  class="state mono"
                  class:ok={isLocal || st?.available}
                  class:off={!isLocal && !st?.available}
                >
                  {isLocal ? 'install from a file' : st?.available ? 'available' : 'not installed'}
                </span>
              </div>
              {#if !isLocal && st && !st.available}
                <button class="btn" onclick={() => install(s)} disabled={busy === s}>
                  {busy === s ? 'Working…' : 'Install'}
                </button>
              {/if}
              <label class="switch" title="Enable {names[s]}">
                <input
                  type="checkbox"
                  checked={$settings.managers[s] !== false}
                  onchange={(e) => setManagerEnabled(s, e.currentTarget.checked)}
                />
                <span class="slider"></span>
              </label>
            </div>
          {/each}
        </div>
      </section>

      {#if scoopAvailable}
        <section class="group source-group">
          <h2>Scoop buckets</h2>
          <p class="muted hint">
            Buckets are app catalogs for Scoop. Some apps (e.g. Firefox, VLC) live in the
            <span class="mono">extras</span> bucket and won't install via Scoop until it's added.
          </p>
          {#if buckets === null}
            <p class="muted">Loading…</p>
          {:else}
            <div class="bucket-list">
              {#each bucketRows as row (row.name)}
                <div class="bucket-row" class:is-added={row.added}>
                  <div class="bucket-meta">
                    <span class="bucket-name mono">{row.name}</span>
                    {#if row.description}<span class="bucket-desc muted">{row.description}</span>{/if}
                  </div>
                  {#if row.added && row.name === 'main'}
                    <span class="bucket-state">Added</span>
                  {:else if row.added}
                    <button
                      class="btn btn-ghost bucket-btn"
                      onclick={() => removeBucket(row.name)}
                      disabled={bucketBusy !== null}
                    >
                      {bucketBusy === row.name ? 'Removing…' : 'Remove'}
                    </button>
                  {:else}
                    <button
                      class="btn bucket-btn"
                      onclick={() => addBucket(row.name)}
                      disabled={bucketBusy !== null}
                    >
                      {bucketBusy === row.name ? 'Adding…' : 'Add'}
                    </button>
                  {/if}
                </div>
              {/each}
            </div>
          {/if}
        </section>
      {/if}

      {#if wingetAvailable || scoopAvailable}
        <section class="group source-group">
          <h2>Maintenance</h2>
          <p class="muted hint">Refresh manager sources and clear out old versions.</p>
          <div class="maint">
            {#if wingetAvailable}
              <button
                class="btn"
                disabled={maintBusy !== null}
                onclick={() => runMaint('Update winget sources', 'winget-src', api.wingetUpdateSources)}
              >
                {maintBusy === 'winget-src' ? 'Working…' : 'Update winget sources'}
              </button>
            {/if}
            {#if scoopAvailable}
              <button
                class="btn"
                disabled={maintBusy !== null}
                onclick={() => runMaint('Update Scoop', 'scoop-up', api.scoopUpdate)}
              >
                {maintBusy === 'scoop-up' ? 'Working…' : 'Update Scoop'}
              </button>
              <button
                class="btn"
                disabled={maintBusy !== null}
                onclick={() => runMaint('Clean up Scoop', 'scoop-clean', api.scoopCleanup)}
              >
                {maintBusy === 'scoop-clean' ? 'Working…' : 'Clean up Scoop'}
              </button>
            {/if}
          </div>
        </section>
      {/if}

      <section class="group source-group">
        <h2>App icons</h2>
        <label class="toggle-row card">
          <span class="toggle-text">
            <span class="toggle-title">Download &amp; cache app icons</span>
            <span class="toggle-sub muted">
              Off by default. Icons are fetched from each app's website as you browse and stored on
              disk, so they load instantly next time. Apps without a known website keep the lettered
              tile.
            </span>
          </span>
          <span class="switch">
            <input
              type="checkbox"
              checked={$settings.downloadIcons}
              onchange={(e) => setDownloadIcons(e.currentTarget.checked)}
            />
            <span class="slider"></span>
          </span>
        </label>
        <button class="btn" onclick={clearIcons} disabled={clearing}>
          {clearing ? 'Clearing…' : 'Clear icon cache'}
        </button>
      </section>

      <section class="group source-group">
        <h2>Curated catalog</h2>
        <p class="muted hint">Edit the categories and apps shown on the Discover home page.</p>
        <a class="btn" href="/curated">Open catalog editor</a>
      </section>
    {:else if activeTab === 'updates'}
      <section class="group">
        <h2>Software updates</h2>
        <p class="muted hint">Acy <span class="mono">v{appVersion || '…'}</span>.</p>

        <label class="toggle-row card">
          <span class="toggle-text">
            <span class="toggle-title">Automatically check for updates</span>
            <span class="toggle-sub muted">
              On startup and periodically in the background. You'll be asked before anything
              downloads.
            </span>
          </span>
          <span class="switch">
            <input
              type="checkbox"
              checked={$settings.autoCheckUpdates}
              onchange={(e) => onAutoCheckToggle(e.currentTarget.checked)}
            />
            <span class="slider"></span>
          </span>
        </label>

        <div class="upd">
          {#if $updaterPhase === 'available'}
            <button class="btn btn-accent" onclick={installUpdate}>
              Download &amp; install v{$updaterVersion}
            </button>
            <p class="upd-msg accent">Version {$updaterVersion} is available.</p>
          {:else if $updaterPhase === 'downloading'}
            <button class="btn" disabled>Downloading…</button>
          {:else}
            <button class="btn" onclick={checkForUpdate} disabled={$updaterPhase === 'checking'}>
              {$updaterPhase === 'checking' ? 'Checking…' : 'Check for updates'}
            </button>
            {#if $updaterPhase === 'uptodate'}
              <p class="upd-msg ok">You're on the latest version.</p>
            {:else if $updaterPhase === 'error'}
              <p class="upd-msg err">Update check failed: {$updaterError}</p>
            {/if}
          {/if}
        </div>
      </section>
    {:else if activeTab === 'about'}
      <section class="group">
        <h2>About</h2>
        <p class="about muted">Acy <span class="mono">v{appVersion || '…'}</span></p>
        <p class="about muted">License: <span class="mono">MIT</span></p>
        <p class="about">
          <a class="link" href="https://github.com/0x89-y" target="_blank" rel="noreferrer noopener">
            github.com/0x89-y
          </a>
        </p>
      </section>

      <section class="group">
        <h2>What's new</h2>
        <div class="log">
          {#each CHANGELOG.slice(0, 1) as rel (rel.version)}
            <div class="rel">
              <div class="rel-head">
                <span class="mono rel-ver">v{rel.version}</span>
                <span class="mono muted rel-date">{rel.date}</span>
              </div>
              <ul class="rel-list">
                {#each rel.changes as c (c)}
                  <li>{c}</li>
                {/each}
              </ul>
            </div>
          {/each}
        </div>
        <a class="btn btn-ghost" href="/changelog">View full changelog</a>
      </section>

      <section class="group">
        <h2>Activity</h2>
        {#if $activity.length === 0}
          <p class="muted hint">Your installs, updates, and removals will show up here.</p>
        {:else}
          <div class="log">
            {#each $activity.slice(0, ACTIVITY_PREVIEW) as a (a.id)}
              <div class="log-row">
                <span class="log-dot" class:ok={a.ok} class:bad={!a.ok}></span>
                <span class="log-txt">
                  <strong>{actionLabel[a.action]}</strong>
                  {a.name}
                  {#if a.source}<span class="mono muted log-src">{a.source}</span>{/if}
                </span>
                <span class="log-time mono muted">{when(a.at)}</span>
              </div>
            {/each}
          </div>
          <div class="log-actions">
            <a class="btn btn-ghost" href="/activity">View activity</a>
          </div>
        {/if}
      </section>

    {/if}
  </div>
</div>

<style>
  .about {
    font-size: 0.9rem;
  }
  .about .link {
    color: var(--accent);
    text-decoration: none;
  }
  .about .link:hover {
    text-decoration: underline;
  }
  h1 {
    margin-bottom: 24px;
  }

  .settings-layout {
    display: flex;
    gap: 30px;
    align-items: flex-start;
  }
  .side {
    position: sticky;
    top: 74px;
    flex: 0 0 160px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .side-link {
    text-align: left;
    padding: 8px 12px;
    border: none;
    border-left: 2px solid transparent;
    background: transparent;
    color: var(--text-muted);
    border-radius: var(--radius-sm);
    font-size: 0.92rem;
    font-weight: 500;
  }
  .side-link:hover {
    background: var(--surface-hover);
    color: var(--text);
  }
  .side-link.active {
    background: var(--surface);
    color: var(--text);
    border-left-color: var(--accent);
  }
  .panes {
    flex: 1;
    min-width: 0;
    max-width: 640px;
  }

  .group {
    margin: 0 0 28px;
  }
  .group:last-child {
    margin-bottom: 0;
  }
  .group h2 {
    font-size: 1.05rem;
    margin-bottom: 12px;
  }
  .hint {
    font-size: 0.86rem;
    margin: -4px 0 14px;
    max-width: 520px;
  }

  .seg {
    display: inline-flex;
    gap: 4px;
    padding: 4px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }
  .seg-btn {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    padding: 8px 16px;
    border: none;
    background: transparent;
    color: var(--text-muted);
    border-radius: var(--radius-sm);
    font-size: 0.9rem;
    font-weight: 500;
  }
  .seg-btn:hover {
    color: var(--text);
  }
  .seg-btn.on {
    background: var(--accent);
    color: var(--accent-contrast);
  }

  .field {
    margin-bottom: 18px;
  }
  .field:last-child {
    margin-bottom: 0;
  }
  .field-label {
    display: block;
    font-size: 0.82rem;
    color: var(--text-muted);
    margin-bottom: 8px;
  }
  .pref {
    display: grid;
    grid-template-columns: 124px minmax(0, 1fr);
    align-items: center;
    gap: 6px 12px;
    margin-bottom: 14px;
  }
  .pref .field-label {
    margin: 0;
  }
  .pref-select {
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    color: var(--text);
    padding: 8px 12px;
    font-size: 0.9rem;
  }
  .pref .hint {
    grid-column: 2;
    margin: 0;
    font-size: 0.78rem;
  }
  .source-group {
    margin-bottom: 20px;
  }
  .source-group h2 {
    margin-bottom: 9px;
  }
  .manager-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 7px;
  }
  .manager-row {
    gap: 9px;
    min-width: 0;
    padding: 9px 11px;
  }
  .manager-row .info {
    flex-direction: column;
    align-items: flex-start;
    gap: 0;
    min-width: 0;
  }
  .manager-row .btn {
    padding: 5px 9px;
    font-size: 0.78rem;
  }
  .bucket-list {
    display: flex;
    flex-direction: column;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    overflow: hidden;
  }
  .bucket-row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 9px 12px;
    border-top: 1px solid var(--border);
  }
  .bucket-row:first-child {
    border-top: none;
  }
  .bucket-row.is-added {
    background: color-mix(in srgb, var(--accent) 5%, transparent);
  }
  .bucket-meta {
    display: flex;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
    flex: 1;
  }
  .bucket-name {
    font-size: 0.84rem;
    font-weight: 600;
    color: var(--text);
  }
  .bucket-desc {
    font-size: 0.76rem;
  }
  .bucket-state {
    font-size: 0.76rem;
    font-weight: 600;
    color: var(--accent);
    flex-shrink: 0;
    padding-right: 4px;
  }
  .bucket-btn {
    font-size: 0.8rem;
    padding: 5px 12px;
    flex-shrink: 0;
    min-width: 76px;
  }
  .maint {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .upd {
    margin-top: 12px;
  }
  .upd-msg {
    font-size: 0.84rem;
    margin: 10px 0 0;
  }
  .upd-msg.ok {
    color: var(--success);
    font-weight: 500;
  }
  .upd-msg.accent {
    color: var(--accent);
    font-weight: 500;
  }
  .upd-msg.err {
    color: var(--danger);
    font-family: var(--font-mono);
    font-size: 0.8rem;
    white-space: pre-wrap;
  }
  .accents {
    display: flex;
    gap: 12px;
  }
  .swatch {
    width: 28px;
    height: 28px;
    border-radius: 50%;
    background: var(--sw);
    border: none;
    cursor: pointer;
    padding: 0;
    transition: transform 0.1s;
  }
  .swatch:hover {
    transform: scale(1.1);
  }
  .swatch.on {
    box-shadow:
      0 0 0 2px var(--bg),
      0 0 0 4px var(--sw);
  }

  .row {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 12px 16px;
  }
  .info {
    flex: 1;
    display: flex;
    align-items: baseline;
    gap: 10px;
  }
  .name {
    font-weight: 600;
  }
  .state {
    font-size: 0.74rem;
  }
  .state.ok {
    color: var(--success);
  }
  .state.off {
    color: var(--text-muted);
  }

  .switch {
    position: relative;
    display: inline-block;
    width: 40px;
    height: 22px;
    flex-shrink: 0;
  }
  .switch input {
    opacity: 0;
    width: 0;
    height: 0;
  }
  .slider {
    position: absolute;
    inset: 0;
    cursor: pointer;
    background: var(--border-strong);
    border-radius: var(--radius-pill);
    transition: background 0.15s;
  }
  .slider::before {
    content: '';
    position: absolute;
    height: 16px;
    width: 16px;
    left: 3px;
    top: 3px;
    background: #fff;
    border-radius: 50%;
    transition: transform 0.15s;
  }
  .switch input:checked + .slider {
    background: var(--accent);
  }
  .switch input:checked + .slider::before {
    transform: translateX(18px);
  }

  .toggle-row {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 14px 16px;
    margin-bottom: 12px;
    cursor: pointer;
  }
  .toggle-text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .toggle-title {
    font-size: 0.92rem;
    font-weight: 500;
  }
  .toggle-sub {
    font-size: 0.8rem;
    line-height: 1.45;
  }

  .log {
    text-align: left;
    margin-bottom: 12px;
  }
  .log-actions {
    display: flex;
    gap: 8px;
  }
  .log-row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 7px 0;
    border-bottom: 1px solid var(--border);
    font-size: 0.88rem;
  }
  .log-row:last-child {
    border-bottom: none;
  }
  .log-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
  }
  .log-dot.ok {
    background: var(--success);
  }
  .log-dot.bad {
    background: var(--danger);
  }
  .log-txt {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .log-src {
    font-size: 0.74rem;
  }
  .log-time {
    font-size: 0.74rem;
    flex-shrink: 0;
  }

  .rel {
    padding: 4px 0 10px;
    border-bottom: 1px solid var(--border);
  }
  .rel:last-child {
    border-bottom: none;
  }
  .rel-head {
    display: flex;
    align-items: baseline;
    gap: 10px;
    margin-bottom: 6px;
  }
  .rel-ver {
    font-weight: 600;
    font-size: 0.9rem;
  }
  .rel-date {
    font-size: 0.74rem;
  }
  .rel-list {
    margin: 0;
    padding-left: 18px;
    font-size: 0.86rem;
    color: var(--text-muted);
  }
  .rel-list li {
    margin-bottom: 3px;
  }

  @media (max-width: 720px) {
    .settings-layout {
      flex-direction: column;
    }
    .side {
      position: static;
      flex: none;
      flex-direction: row;
      flex-wrap: wrap;
      gap: 4px;
      margin-bottom: 8px;
    }
    .manager-grid {
      grid-template-columns: 1fr;
    }
    .pref {
      grid-template-columns: 1fr;
    }
    .pref .hint {
      grid-column: 1;
    }
  }
</style>

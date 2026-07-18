<script lang="ts">
  import { onMount } from 'svelte';
  import { getVersion } from '@tauri-apps/api/app';
  import {
    Sun,
    Moon,
    Monitor,
    Pipette,
    ArrowLeft,
    RefreshCw,
    Trash2,
    Link,
    FileText,
    SquarePen
  } from '@lucide/svelte';
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
    setSteamGridKey,
    setCloseToTray,
    setNotifyUpdates,
    setNotifyOperations,
    setRefreshOnStartup,
    setAutoCheckUpdates,
    setShowCuratedApps,
    setSettingsTab,
    restartSetup,
    resetSettings,
    setCustomAccent,
    ACCENTS,
    type ThemeMode,
    type SettingsTab
  } from '$lib/stores/settings';
  import { managers, loadManagers } from '$lib/stores/managers';
  import { clearIconCache, iconRefetch, refetchMissingIcons } from '$lib/stores/icons';
  import { reloadCurated } from '$lib/stores/curated';
  import { confirmAction } from '$lib/stores/confirm';
  import { enqueue } from '$lib/stores/ops';
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

  // One-line descriptions shown under each manager in the Sources list.
  const MANAGER_INFO: Record<Source, string> = {
    winget: "Windows Package Manager - Microsoft's built-in catalog",
    scoop: 'Portable apps and developer tools',
    choco: 'Chocolatey - large community catalog',
    msstore: 'Microsoft Store apps',
    local: 'Install directly from a downloaded .exe or .msi'
  };

  let busy = $state<Source | null>(null);
  let clearing = $state(false);
  // Icon re-fetch progress lives in the shared store so it survives leaving and
  // re-entering Settings while a run is in flight.
  let iconRefetching = $derived($iconRefetch.status === 'running');
  let catalogPhase = $state<'idle' | 'checking' | 'available' | 'applying'>('idle');
  let catalogMsg = $state('');
  let catalogVersion = $state(0);
  // Custom catalog (a user-supplied file or URL that replaces the official one).
  let customCat = $state<api.CustomCatalogInfo | null>(null);
  let customUrl = $state('');
  let customMsg = $state('');
  let customBusy = $state(false);
  let appVersion = $state('');
  /** Number of recent activity rows shown in the About preview. */
  const ACTIVITY_PREVIEW = 6;

  onMount(async () => {
    try {
      appVersion = await getVersion();
    } catch {
      appVersion = '';
    }
    try {
      customCat = await api.customCatalogInfo();
    } catch {
      customCat = null;
    }
  });

  /** Load a custom catalog from a file path or URL, validate + apply it, and
   * refresh the shared catalog so Discover reflects it. */
  async function applyCustomCatalog(source: string, isUrl: boolean) {
    customBusy = true;
    customMsg = '';
    try {
      customCat = await api.setCustomCatalog(source, isUrl);
      customUrl = '';
      await reloadCurated();
      customMsg = `Loaded ${customCat.appCount} apps (catalog v${customCat.version}).`;
    } catch (e) {
      customMsg = typeof e === 'string' ? e : 'Could not load that catalog.';
    }
    customBusy = false;
  }

  async function chooseCustomFile() {
    const path = await api.pickCatalogFile();
    if (path) await applyCustomCatalog(path, false);
  }

  async function useCustomUrl() {
    const url = customUrl.trim();
    if (url) await applyCustomCatalog(url, true);
  }

  async function refreshCustomCatalog() {
    if (customCat) await applyCustomCatalog(customCat.source, customCat.isUrl);
  }

  async function removeCustomCatalog() {
    customBusy = true;
    customMsg = '';
    try {
      await api.clearCustomCatalog();
      customCat = null;
      await reloadCurated();
      customMsg = 'Reverted to the official catalog.';
    } catch (e) {
      customMsg = typeof e === 'string' ? e : 'Could not remove the custom catalog.';
    }
    customBusy = false;
  }

  async function clearIcons() {
    clearing = true;
    await clearIconCache();
    clearing = false;
  }

  /** Check for a newer hosted catalog (signature-verified in Rust) without
   * applying it - like the app updater, it only reports what's available. */
  async function checkCatalog() {
    catalogPhase = 'checking';
    catalogMsg = '';
    try {
      const res = await api.updateCuratedCatalog(false);
      if (res.available) {
        catalogVersion = res.version;
        catalogPhase = 'available';
      } else {
        catalogMsg = res.message;
        catalogPhase = 'idle';
      }
    } catch (e) {
      catalogMsg = typeof e === 'string' ? e : 'Catalog check failed.';
      catalogPhase = 'idle';
    }
  }

  /** Apply the available catalog update, then refresh Discover. */
  async function applyCatalog() {
    catalogPhase = 'applying';
    try {
      const res = await api.updateCuratedCatalog(true);
      if (res.updated) await reloadCurated();
      catalogMsg = res.message;
    } catch (e) {
      catalogMsg = typeof e === 'string' ? e : 'Catalog update failed.';
    }
    catalogPhase = 'idle';
  }

  async function resetAll() {
    const ok = await confirmAction({
      title: 'Reset all settings?',
      message:
        'Theme, accent, managers, and all preferences go back to their defaults. ' +
        'Your installed apps and curated list are not affected.',
      confirmLabel: 'Reset settings',
      danger: true
    });
    if (ok) resetSettings();
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

  // ---- Manager maintenance ----
  let maintBusy = $state<string | null>(null);
  async function runMaint(label: string, k: string, fn: (opId: string) => Promise<number>) {
    maintBusy = k;
    await enqueue(label, fn);
    maintBusy = null;
  }

  // App self-update is driven by the updater store (see imports). Enabling
  // auto-check kicks off an immediate background check.
  function onAutoCheckToggle(on: boolean) {
    setAutoCheckUpdates(on);
    if (on) backgroundCheck();
  }

  const tabs: { id: SettingsTab; label: string }[] = [
    { id: 'general', label: 'General' },
    { id: 'appearance', label: 'Appearance' },
    { id: 'sources', label: 'Sources' },
    { id: 'updates', label: 'Updates' },
    { id: 'about', label: 'About' }
  ];
  let activeTab = $derived($settings.settingsTab);
</script>

<div class="settings-layout">
  <nav class="side" aria-label="Settings sections">
    <div class="rail-head">
      <a class="back-btn" href="/" title="Back" aria-label="Back"><ArrowLeft size={17} /></a>
      <span class="rail-title">Settings</span>
    </div>
    <div class="rail-links">
      {#each tabs as t (t.id)}
        <button class="side-link" class:active={activeTab === t.id} onclick={() => setSettingsTab(t.id)}>
          {t.label}
        </button>
      {/each}
    </div>
  </nav>

  <div class="panes">
    {#if activeTab === 'general'}
      <section class="group">
        <h2>Behavior</h2>
        <div class="opt-list">
          <label class="opt-row">
            <span class="opt-label">Show command output while installing</span>
            <span class="switch">
              <input
                type="checkbox"
                checked={$settings.showOutput}
                onchange={(e) => setShowOutput(e.currentTarget.checked)}
              />
              <span class="slider"></span>
            </span>
          </label>
        </div>
      </section>

      <section class="group">
        <h2>Tray &amp; notifications</h2>
        <div class="opt-list">
          <label class="opt-row">
            <span class="opt-label">Close to tray instead of quitting</span>
            <span class="switch">
              <input
                type="checkbox"
                checked={$settings.closeToTray}
                onchange={(e) => setCloseToTray(e.currentTarget.checked)}
              />
              <span class="slider"></span>
            </span>
          </label>
          <label class="opt-row">
            <span class="opt-label">Notify when long operations finish</span>
            <span class="switch">
              <input
                type="checkbox"
                checked={$settings.notifyOperations}
                onchange={(e) => setNotifyOperations(e.currentTarget.checked)}
              />
              <span class="slider"></span>
            </span>
          </label>
        </div>
      </section>

      <section class="group">
        <h2>Setup &amp; reset</h2>
        <div class="reset-row">
          <button class="btn btn-accent" onclick={restartSetup}>Run setup again</button>
          <button class="btn" onclick={resetAll}>Reset all settings</button>
        </div>
      </section>
    {:else if activeTab === 'appearance'}
      <section class="group">
        <h2>Theme &amp; accent</h2>
        <div class="opt-list">
          <div class="opt-row">
            <span class="opt-label">Theme</span>
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
          <div class="opt-row">
            <span class="opt-label">Accent</span>
            <div class="accents">
              {#each ACCENTS as a (a.name)}
                <button
                  class="swatch"
                  class:on={$settings.accent === a.name}
                  class:aurora={a.name === 'aurora'}
                  style="--sw:{a.color}"
                  onclick={() => setAccent(a.name)}
                  title={a.label}
                  aria-label={a.label}
                ></button>
              {/each}
              <label
                class="swatch custom"
                class:on={$settings.accent === 'custom'}
                style="--sw:{$settings.customAccent}"
                title="Custom colour"
              >
                <input
                  type="color"
                  value={$settings.customAccent}
                  oninput={(e) => setCustomAccent(e.currentTarget.value)}
                  aria-label="Custom accent colour"
                />
                <Pipette size={13} class="pip" />
              </label>
            </div>
          </div>
        </div>
      </section>

      <section class="group">
        <div class="group-head">
          <h2>App icons</h2>
          <div class="head-actions">
            {#if $iconRefetch.status === 'running' && $iconRefetch.total > 0}
              <span class="icon-msg muted"
                >{Math.min($iconRefetch.current + 1, $iconRefetch.total)} / {$iconRefetch.total}</span
              >
            {:else if $iconRefetch.status === 'done'}
              <span class="icon-msg muted">{$iconRefetch.message}</span>
            {/if}
            <button
              class="icon-btn"
              title="Re-download missing icons"
              aria-label="Re-download missing icons"
              onclick={refetchMissingIcons}
              disabled={!$settings.downloadIcons || iconRefetching || clearing}
            >
              <RefreshCw size={16} class={iconRefetching ? 'icon-spin' : ''} />
            </button>
            <button
              class="icon-btn"
              title="Clear icon cache"
              aria-label="Clear icon cache"
              onclick={clearIcons}
              disabled={clearing || iconRefetching}
            >
              <Trash2 size={16} />
            </button>
          </div>
        </div>
        <div class="opt-list">
          <label class="opt-row">
            <span class="opt-label">Download &amp; cache app icons</span>
            <span class="switch">
              <input
                type="checkbox"
                checked={$settings.downloadIcons}
                onchange={(e) => setDownloadIcons(e.currentTarget.checked)}
              />
              <span class="slider"></span>
            </span>
          </label>
          <div class="key-row">
            <div class="key-head">
              <span class="opt-label">SteamGridDB API key</span>
              <span class="key-hint muted"
                >Optional - fetches icons for Steam and other launcher games (Epic, GOG, EA,
                Ubisoft, Battle.net).</span
              >
            </div>
            <input
              class="key-input mono"
              type="password"
              autocomplete="off"
              spellcheck="false"
              placeholder="Paste key…"
              value={$settings.steamGridKey}
              onchange={(e) => setSteamGridKey(e.currentTarget.value)}
            />
          </div>
        </div>
      </section>
    {:else if activeTab === 'sources'}
      <section class="group source-group">
        <h2>Package managers</h2>
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
        </div>
        <div class="mgr-list">
          {#each allManagers as s (s)}
            {@const st = statusOf(s)}
            {@const isLocal = s === 'local'}
            <div class="mgr-row" class:is-on={$settings.managers[s] !== false}>
              <div class="mgr-meta">
                <span class="mgr-name">{names[s]}</span>
                <span class="mgr-desc muted">{MANAGER_INFO[s]}</span>
              </div>
              <div class="mgr-actions">
                {#if s === 'winget' && st?.available}
                  <button
                    class="icon-btn"
                    title="Update winget sources"
                    aria-label="Update winget sources"
                    disabled={maintBusy !== null}
                    onclick={() =>
                      runMaint('Update winget sources', 'winget-src', api.wingetUpdateSources)}
                  >
                    <RefreshCw size={16} class={maintBusy === 'winget-src' ? 'icon-spin' : ''} />
                  </button>
                {/if}
                {#if s === 'scoop'}
                  {#if st?.available}
                    <a class="btn btn-ghost mgr-btn" href="/scoop-buckets">Manage buckets</a>
                    <button
                      class="icon-btn"
                      title="Update Scoop"
                      aria-label="Update Scoop"
                      disabled={maintBusy !== null}
                      onclick={() => runMaint('Update Scoop', 'scoop-up', api.scoopUpdate)}
                    >
                      <RefreshCw size={16} class={maintBusy === 'scoop-up' ? 'icon-spin' : ''} />
                    </button>
                    <button
                      class="icon-btn"
                      title="Clean up Scoop"
                      aria-label="Clean up Scoop"
                      disabled={maintBusy !== null}
                      onclick={() => runMaint('Clean up Scoop', 'scoop-clean', api.scoopCleanup)}
                    >
                      <Trash2 size={16} />
                    </button>
                  {:else}
                    <button class="btn btn-ghost mgr-btn" disabled title="Install Scoop first">
                      Manage buckets
                    </button>
                  {/if}
                {/if}
                {#if !isLocal && st && !st.available}
                  <button class="btn mgr-btn" onclick={() => install(s)} disabled={busy === s}>
                    {busy === s ? 'Working…' : 'Install'}
                  </button>
                {:else}
                  <span
                    class="mgr-state mono"
                    class:ok={isLocal || st?.available}
                    class:off={!isLocal && !st?.available}
                  >
                    {isLocal ? 'file-based' : st?.available ? 'available' : 'not installed'}
                  </span>
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
            </div>
          {/each}
        </div>
      </section>

      <section class="group source-group">
        <div class="group-head">
          <h2>Curated catalog</h2>
          <div class="head-actions">
            {#if !customCat && catalogPhase !== 'available' && catalogMsg}
              <span class="icon-msg muted">{catalogMsg}</span>
            {/if}
            <a
              class="icon-btn"
              href="/curated"
              title="Open catalog editor"
              aria-label="Open catalog editor"
            >
              <SquarePen size={16} />
            </a>
            {#if !customCat}
              <button
                class="icon-btn"
                title="Check for catalog updates"
                aria-label="Check for catalog updates"
                onclick={checkCatalog}
                disabled={catalogPhase === 'checking' || catalogPhase === 'applying'}
              >
                <RefreshCw
                  size={16}
                  class={catalogPhase === 'checking' || catalogPhase === 'applying'
                    ? 'icon-spin'
                    : ''}
                />
              </button>
            {/if}
          </div>
        </div>
        <div class="opt-list">
          <label class="opt-row">
            <span class="opt-label">Show curated apps</span>
            <span class="switch">
              <input
                type="checkbox"
                checked={$settings.showCuratedApps}
                onchange={(e) => setShowCuratedApps(e.currentTarget.checked)}
              />
              <span class="slider"></span>
            </span>
          </label>
        </div>
        {#if catalogPhase === 'available'}
          <div class="icon-actions">
            <div class="seg-actions">
              <button class="seg-act" onclick={applyCatalog}>Update to v{catalogVersion}</button>
            </div>
            <span class="icon-msg accent-msg">Catalog v{catalogVersion} is available.</span>
          </div>
        {/if}
        <div class="custom-cat">
          {#if customCat}
            <div class="custom-active">
              <span class="custom-src">
                {#if customCat.isUrl}<Link size={14} />{:else}<FileText size={14} />{/if}
                <span class="custom-path mono">{customCat.source}</span>
              </span>
              <span class="custom-meta muted">v{customCat.version} · {customCat.appCount} apps</span
              >
            </div>
            <div class="seg-actions">
              <button class="seg-act" onclick={refreshCustomCatalog} disabled={customBusy}>
                {customBusy ? 'Working…' : 'Refresh'}
              </button>
              <button class="seg-act" onclick={removeCustomCatalog} disabled={customBusy}>
                Remove
              </button>
            </div>
          {:else}
            <span class="opt-label">Use custom catalog</span>
            <div class="custom-row">
              <button class="seg-act" onclick={chooseCustomFile} disabled={customBusy}>
                Choose file…
              </button>
              <input
                class="key-input mono custom-input"
                type="text"
                placeholder="https://…/curated.json"
                autocomplete="off"
                spellcheck="false"
                bind:value={customUrl}
                onkeydown={(e) => e.key === 'Enter' && useCustomUrl()}
              />
              <button
                class="seg-act"
                onclick={useCustomUrl}
                disabled={customBusy || !customUrl.trim()}
              >
                Use URL
              </button>
            </div>
          {/if}
          {#if customMsg}<span class="icon-msg muted">{customMsg}</span>{/if}
        </div>
      </section>
    {:else if activeTab === 'updates'}
      <section class="group">
        <h2>App updates</h2>
        <div class="opt-list">
          <label class="opt-row">
            <span class="opt-label">Check for updates on startup</span>
            <span class="switch">
              <input
                type="checkbox"
                checked={$settings.refreshOnStartup}
                onchange={(e) => setRefreshOnStartup(e.currentTarget.checked)}
              />
              <span class="slider"></span>
            </span>
          </label>
          <label class="opt-row">
            <span class="opt-label">Notify when new updates are found</span>
            <span class="switch">
              <input
                type="checkbox"
                checked={$settings.notifyUpdates}
                onchange={(e) => setNotifyUpdates(e.currentTarget.checked)}
              />
              <span class="slider"></span>
            </span>
          </label>
        </div>
      </section>

      <section class="group">
        <div class="group-head">
          <h2>Acy updates</h2>
          <div class="head-actions">
            {#if $updaterPhase === 'available'}
              <span class="head-status accent">v{$updaterVersion} available</span>
            {:else if $updaterPhase === 'uptodate'}
              <span class="head-status ok">You're on the latest version.</span>
            {:else if $updaterPhase === 'error'}
              <span class="head-status err" title="Update check failed: {$updaterError}"
                >Update check failed</span
              >
            {/if}
            <span class="head-ver muted">Acy <span class="mono">v{appVersion || '…'}</span></span>
            <button
              class="icon-btn"
              title="Check for updates"
              aria-label="Check for updates"
              onclick={checkForUpdate}
              disabled={$updaterPhase === 'checking' || $updaterPhase === 'downloading'}
            >
              <RefreshCw size={16} class={$updaterPhase === 'checking' ? 'icon-spin' : ''} />
            </button>
          </div>
        </div>

        <div class="opt-list">
          <label class="opt-row">
            <span class="opt-label">Automatically check for Acy updates</span>
            <span class="switch">
              <input
                type="checkbox"
                checked={$settings.autoCheckUpdates}
                onchange={(e) => onAutoCheckToggle(e.currentTarget.checked)}
              />
              <span class="slider"></span>
            </span>
          </label>
        </div>

        {#if $updaterPhase === 'available'}
          <div class="upd">
            <button class="btn btn-accent" onclick={installUpdate}>
              Download &amp; install v{$updaterVersion}
            </button>
          </div>
        {:else if $updaterPhase === 'downloading'}
          <div class="upd">
            <button class="btn btn-accent" disabled>Downloading…</button>
          </div>
        {/if}
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
    padding: 0 var(--settings-pad);
  }
  .about .link {
    color: var(--accent);
    text-decoration: none;
  }
  .about .link:hover {
    text-decoration: underline;
  }
  /* Full-bleed master-detail panel: the settings rail on the left, the active
     section's content on the right, filling the window like the home screen. */
  .settings-layout {
    flex: 1;
    min-height: 0;
    display: flex;
    align-items: stretch;
    overflow: hidden;
    background: var(--surface);
  }
  .side {
    flex: 0 0 190px;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    border-right: 1px solid var(--border);
    background: var(--surface-2);
  }
  /* Back button + title live at the top of the rail (like Discover's switch). */
  .rail-head {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
  }
  .rail-title {
    font-size: 0.95rem;
    font-weight: 600;
  }
  .back-btn {
    flex-shrink: 0;
    width: 30px;
    height: 30px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface);
    color: var(--text-muted);
    line-height: 0;
  }
  .back-btn:hover {
    background: var(--surface-hover);
    color: var(--text);
    border-color: var(--accent);
  }
  .rail-links {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
  }
  .side-link {
    text-align: left;
    padding: 10px 14px;
    border: none;
    border-top: 1px solid var(--border);
    border-left: 2px solid transparent;
    background: transparent;
    color: var(--text-muted);
    border-radius: 0;
    font-size: 0.92rem;
    font-weight: 500;
  }
  .side-link:first-child {
    border-top: none;
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
    min-height: 0;
    overflow-y: auto;
    --settings-pad: 20px;
    display: flex;
    flex-direction: column;
    padding: 24px 0;
  }

  .group {
    margin: 0;
  }
  /* Full-width divider between adjacent sections. */
  .group + .group {
    border-top: 1px solid var(--border);
    padding-top: 20px;
  }
  .group h2 {
    font-size: 1.05rem;
    font-weight: 600;
    margin: 0;
    padding: 0 var(--settings-pad) 10px;
  }
  /* Section header with inline actions (e.g. App icons). */
  .group-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 0 var(--settings-pad) 10px;
  }
  .group-head h2 {
    padding: 0;
  }
  .head-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
  }
  /* Version label sitting next to a header action (Acy updates). */
  .head-ver {
    font-size: 0.86rem;
  }
  /* Updater status shown inline in the Acy-updates header. */
  .head-status {
    font-size: 0.86rem;
    font-weight: 500;
  }
  .head-status.ok {
    color: var(--success);
  }
  .head-status.accent {
    color: var(--accent);
  }
  .head-status.err {
    color: var(--danger);
  }
  .icon-btn {
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    border-radius: 8px;
    border: 1px solid var(--border);
    background: var(--surface-2);
    color: var(--text-muted);
    cursor: pointer;
  }
  .icon-btn:hover:not(:disabled) {
    color: var(--text);
    border-color: var(--accent);
  }
  .icon-btn:disabled {
    opacity: 0.5;
    cursor: default;
  }
  :global(.icon-spin) {
    animation: icon-spin 0.8s linear infinite;
  }
  @keyframes icon-spin {
    to {
      transform: rotate(360deg);
    }
  }
  .hint {
    margin: 0;
    padding: 0 var(--settings-pad) 12px;
    max-width: 520px;
    font-size: 0.86rem;
  }

  .seg {
    display: inline-flex;
    align-items: stretch;
    gap: 2px;
    padding: 2px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    width: fit-content;
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
    background: var(--accent-fill);
    color: var(--accent-contrast);
  }

  .field {
    padding: 12px var(--settings-pad);
    border-top: 1px solid var(--border);
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
  .mgr-list {
    display: flex;
    flex-direction: column;
  }
  .mgr-row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px var(--settings-pad);
    border-top: 1px solid var(--border);
  }
  .mgr-row.is-on {
    background: var(--surface-2);
  }
  .mgr-meta {
    display: flex;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
    flex: 1;
  }
  .mgr-name {
    font-size: 0.84rem;
    font-weight: 600;
    color: var(--text);
  }
  .mgr-desc {
    font-size: 0.76rem;
  }
  .mgr-actions {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-shrink: 0;
  }
  .mgr-state {
    font-size: 0.74rem;
  }
  .mgr-state.ok {
    color: var(--success);
  }
  .mgr-state.off {
    color: var(--text-muted);
  }
  .mgr-btn {
    font-size: 0.8rem;
    padding: 5px 12px;
    flex-shrink: 0;
  }
  /* Free-content rows in the edge-to-edge frame: inset + a hairline above. */
  .icon-actions,
  .reset-row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    padding: 12px var(--settings-pad);
    border-top: 1px solid var(--border);
  }
  .upd {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 8px;
    padding: 12px var(--settings-pad);
    border-top: 1px solid var(--border);
  }
  .icon-msg {
    font-size: 0.85rem;
  }
  .accent-msg {
    color: var(--accent);
    font-weight: 500;
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
    transition: opacity 0.12s;
  }
  .swatch:hover {
    opacity: 0.8;
  }
  .swatch.aurora {
    background: var(--aurora-gradient);
  }
  .swatch.custom {
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
    color: #fff;
  }
  .swatch.custom input {
    position: absolute;
    inset: 0;
    opacity: 0;
    border: none;
    padding: 0;
    cursor: pointer;
  }
  .swatch.custom :global(.pip) {
    pointer-events: none;
    filter: drop-shadow(0 0 1px rgba(0, 0, 0, 0.65));
  }
  .swatch.on {
    box-shadow:
      0 0 0 2px var(--bg),
      0 0 0 4px var(--sw);
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
    background: var(--accent-fill);
  }
  .switch input:checked + .slider::before {
    transform: translateX(18px);
  }

  /* Card-less: each row is separated by a full-width hairline and inset by the
     settings pad (no bordered box). */
  .opt-list {
    display: flex;
    flex-direction: column;
  }
  .opt-list > * {
    padding: 12px var(--settings-pad);
    border-top: 1px solid var(--border);
  }
  .opt-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 14px;
    cursor: pointer;
  }
  .opt-label {
    font-size: 0.9rem;
    font-weight: 500;
    min-width: 0;
  }
  .key-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 14px;
    /* horizontal inset comes from `.opt-list > *` so it lines up with the rows above */
  }
  .key-head {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .key-hint {
    font-size: 0.78rem;
  }
  .key-input {
    flex: 0 0 200px;
    max-width: 200px;
    padding: 6px 9px;
    font-size: 0.82rem;
    color: var(--text);
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 8px;
  }
  .key-input:focus {
    outline: none;
    border-color: var(--accent);
  }

  /* Custom catalog block (Sources → Curated catalog). */
  .custom-cat {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 12px var(--settings-pad);
    border-top: 1px solid var(--border);
  }
  .custom-active {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    flex-wrap: wrap;
  }
  .custom-src {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    color: var(--text);
  }
  .custom-path {
    font-size: 0.82rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 380px;
  }
  .custom-meta {
    font-size: 0.8rem;
    flex-shrink: 0;
  }
  .custom-row {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }
  .custom-input {
    flex: 1 1 220px;
    max-width: 320px;
  }

  .log {
    text-align: left;
    padding: 4px var(--settings-pad) 12px;
  }
  .log-actions {
    display: flex;
    gap: 8px;
    padding: 0 var(--settings-pad) 12px;
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
      flex: none;
      border-right: none;
      border-bottom: 1px solid var(--border);
    }
    .rail-links {
      flex-direction: row;
      flex-wrap: wrap;
      overflow: visible;
    }
    .side-link {
      border-top: none;
      border-left: none;
    }
    .pref {
      grid-template-columns: 1fr;
    }
  }
</style>

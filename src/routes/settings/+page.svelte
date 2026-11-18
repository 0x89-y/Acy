<script lang="ts">
  import { onMount } from 'svelte';
  import { getVersion } from '@tauri-apps/api/app';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import {
    Sun,
    Moon,
    Monitor,
    Pipette,
    RefreshCw,
    Trash2,
    Link,
    FileText,
    SquarePen,
    Terminal,
    FolderDown,
    AppWindow,
    Bell,
    BellRing,
    WandSparkles,
    RotateCcw,
    SunMoon,
    Palette,
    Layers,
    Image,
    KeyRound,
    Star,
    Package,
    BookOpen,
    Sparkles,
    ExternalLink,
    ChevronDown,
    ScrollText
  } from '@lucide/svelte';
  import SettingCard from '$lib/components/SettingCard.svelte';
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
    setInstallLocation,
    setDownloadIcons,
    setSteamGridKey,
    setCloseToTray,
    setNotifyUpdates,
    setNotifyOperations,
    setRefreshOnStartup,
    setAutoCheckUpdates,
    setShowCuratedApps,
    setBackdrop,
    BACKDROPS,
    SETTINGS_TABS,
    restartSetup,
    resetSettings,
    setCustomAccent,
    ACCENTS,
    type ThemeMode
  } from '$lib/stores/settings';
  import { managers, loadManagers } from '$lib/stores/managers';
  import {
    clearIconCache,
    iconCacheVersion,
    iconRefetch,
    refetchMissingIcons
  } from '$lib/stores/icons';
  import { catalog, downloadCatalog, loadCatalogStatus, reloadCurated } from '$lib/stores/curated';
  import { confirmAction } from '$lib/stores/confirm';
  import { enqueue } from '$lib/stores/ops';
  import { CHANGELOG } from '$lib/changelog';
  import ShortcutList from '$lib/components/ShortcutList.svelte';
  import * as api from '$lib/api';
  import type { Source } from '$lib/types';

  const modes: { value: ThemeMode; label: string; icon: typeof Sun }[] = [
    { value: 'light', label: 'Light', icon: Sun },
    { value: 'dark', label: 'Dark', icon: Moon },
    { value: 'system', label: 'System', icon: Monitor }
  ];

  const names: Record<Source, string> = {
    winget: 'winget',
    scoop: 'Scoop',
    choco: 'Chocolatey',
    msstore: 'Microsoft Store'
  };
  const allManagers: Source[] = ['winget', 'scoop', 'choco', 'msstore'];

  const MANAGER_INFO: Record<Source, string> = {
    winget: "Windows Package Manager - Microsoft's built-in catalog",
    scoop: 'Portable apps and developer tools',
    choco: 'Chocolatey - large community catalog',
    msstore: 'Microsoft Store apps'
  };

  let busy = $state<Source | null>(null);
  let clearing = $state(false);
  let iconRefetching = $derived($iconRefetch.status === 'running');

  let iconStats = $state<{ icons: number; bytes: number } | null>(null);
  // Re-read whenever the cache is cleared or icons are (re)downloaded.
  $effect(() => {
    void $iconCacheVersion;
    api
      .iconCacheStats()
      .then((s) => (iconStats = s))
      .catch(() => {});
  });

  function formatBytes(n: number): string {
    if (n < 1024) return `${n} B`;
    const kb = n / 1024;
    return kb < 1024 ? `${Math.round(kb)} KB` : `${(kb / 1024).toFixed(1)} MB`;
  }
  let catalogPhase = $state<'idle' | 'checking' | 'available' | 'applying'>('idle');
  let catalogMsg = $state('');
  let catalogVersion = $state(0);
  let customCat = $state<api.CustomCatalogInfo | null>(null);
  let customUrl = $state('');
  let customMsg = $state('');
  let customBusy = $state(false);
  let appVersion = $state('');

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
    loadCatalogStatus();
  });

  function toggleCurated(on: boolean) {
    setShowCuratedApps(on);
    if (on) downloadCatalog();
    else loadCatalogStatus();
  }

  async function applyCustomCatalog(source: string, isUrl: boolean) {
    customBusy = true;
    customMsg = '';
    try {
      customCat = await api.setCustomCatalog(source, isUrl);
      customUrl = '';
      await reloadCurated();
      await loadCatalogStatus();
      customMsg = `Loaded ${customCat.appCount} apps (catalog v${customCat.version}).`;
    } catch (e) {
      customMsg = typeof e === 'string' ? e : 'Could not load that catalog.';
    }
    customBusy = false;
  }

  async function chooseInstallLocation() {
    const path = await api.pickFolder();
    if (path) setInstallLocation(path);
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
      await loadCatalogStatus();
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

  async function applyCatalog() {
    catalogPhase = 'applying';
    try {
      const res = await api.updateCuratedCatalog(true);
      if (res.updated) await reloadCurated();
      await loadCatalogStatus();
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

  let maintBusy = $state<string | null>(null);
  async function runMaint(label: string, k: string, fn: (opId: string) => Promise<number>) {
    maintBusy = k;
    await enqueue(label, fn);
    maintBusy = null;
  }

  function onAutoCheckToggle(on: boolean) {
    setAutoCheckUpdates(on);
    if (on) backgroundCheck();
  }

  let activeTab = $derived($settings.settingsTab);

  const TAB_INTRO: Record<string, string> = {
    general: 'How installs behave, the tray, and resetting Acy.',
    appearance: 'Theme, accent colour, window backdrop and app icons.',
    sources: 'The package managers Acy uses, and where its app catalog comes from.',
    updates: 'Checking your apps for updates, and updating Acy itself.',
    shortcuts: 'Keys that work anywhere in Acy.',
    about: 'Version, licence and what changed recently.'
  };

  function managerState(source: Source): { label: string; kind: 'ok' | 'warn' | 'off' } {
    const st = statusOf(source);
    if (!st) return { label: 'Checking…', kind: 'off' };
    if (!st.available) return { label: 'Not installed', kind: 'off' };
    if (st.needsSetup) return { label: 'Needs setup', kind: 'warn' };
    return { label: 'Installed', kind: 'ok' };
  }
</script>


{#snippet toggleSwitch(checked: boolean, onchange: (on: boolean) => void, label?: string)}
  <span class="switch">
    <input type="checkbox" {checked} aria-label={label} onchange={(e) => onchange(e.currentTarget.checked)} />
    <span class="slider" aria-hidden="true"></span>
  </span>
{/snippet}

<div class="view">
  <header class="view-head">
    <div>
      <h1>{SETTINGS_TABS.find((t) => t.id === activeTab)?.label ?? 'Settings'}</h1>
      <p class="view-sub">{TAB_INTRO[activeTab]}</p>
    </div>
  </header>

  <div class="view-body">
    <div class="page">
      {#if activeTab === 'general'}
        <section class="group" aria-labelledby="h-installing">
          <h2 id="h-installing">Installing</h2>
          <SettingCard
            toggle
            icon={Terminal}
            title="Show command output"
            description="Open the live log for every install, update and removal in the progress bar."
          >
            {#snippet control()}{@render toggleSwitch($settings.showOutput, setShowOutput)}{/snippet}
          </SettingCard>
          <SettingCard icon={FolderDown} title="Install location">
            {#snippet description()}
              {#if $settings.installLocation}
                <span class="mono path" title={$settings.installLocation}>{$settings.installLocation}\&lt;app&gt;</span>
              {:else}
                Installer default (usually Program Files).
              {/if}
              winget only; each app gets its own subfolder. Store/MSIX apps and installers without
              folder support keep their default, and updates stay where the app is.
            {/snippet}
            {#snippet control()}
              <button class="btn" onclick={chooseInstallLocation}>Choose folder…</button>
              {#if $settings.installLocation}
                <button class="btn btn-ghost" onclick={() => setInstallLocation('')}>Reset</button>
              {/if}
            {/snippet}
          </SettingCard>
        </section>

        <section class="group" aria-labelledby="h-window">
          <h2 id="h-window">Window &amp; notifications</h2>
          <SettingCard
            toggle
            icon={AppWindow}
            title="Close to tray"
            description="Closing the window keeps Acy running in the system tray instead of quitting."
          >
            {#snippet control()}{@render toggleSwitch($settings.closeToTray, setCloseToTray)}{/snippet}
          </SettingCard>
          <SettingCard
            toggle
            icon={Bell}
            title="Notify when long operations finish"
            description="A Windows notification when an install or update that took a while is done and Acy is in the background."
          >
            {#snippet control()}{@render toggleSwitch($settings.notifyOperations, setNotifyOperations)}{/snippet}
          </SettingCard>
        </section>

        <section class="group" aria-labelledby="h-reset">
          <h2 id="h-reset">Setup &amp; reset</h2>
          <SettingCard
            icon={WandSparkles}
            title="Run setup again"
            description="Go through the first-run setup: package managers, theme and the app catalog."
          >
            {#snippet control()}<button class="btn" onclick={restartSetup}>Run setup</button>{/snippet}
          </SettingCard>
          <SettingCard
            danger
            icon={RotateCcw}
            title="Reset all settings"
            description="Every preference goes back to its default. Installed apps and your app list are not touched."
          >
            {#snippet control()}<button class="btn danger-outline" onclick={resetAll}>Reset…</button>{/snippet}
          </SettingCard>
        </section>
      {:else if activeTab === 'appearance'}
        <section class="group" aria-labelledby="h-look">
          <h2 id="h-look">Look</h2>
          <SettingCard icon={SunMoon} title="Theme" titleId="theme-label" description="Follow Windows, or always use light or dark.">
            {#snippet control()}
              <div class="seg" role="group" aria-labelledby="theme-label">
                {#each modes as m (m.value)}
                  {@const Icon = m.icon}
                  <button
                    class="seg-btn"
                    class:on={$settings.themeMode === m.value}
                    aria-pressed={$settings.themeMode === m.value}
                    onclick={() => setThemeMode(m.value)}
                  >
                    <Icon size={15} />
                    {m.label}
                  </button>
                {/each}
              </div>
            {/snippet}
          </SettingCard>
          <SettingCard icon={Palette} title="Accent colour" description="Used for buttons, selection and highlights.">
            {#snippet control()}
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
                    aria-pressed={$settings.accent === a.name}
                  ></button>
                {/each}
                <label class="swatch custom" class:on={$settings.accent === 'custom'} style="--sw:{$settings.customAccent}" title="Custom colour">
                  <input
                    type="color"
                    value={$settings.customAccent}
                    oninput={(e) => setCustomAccent(e.currentTarget.value)}
                    aria-label="Custom accent colour"
                  />
                  <Pipette size={13} class="pip" />
                </label>
              </div>
            {/snippet}
          </SettingCard>
          <SettingCard icon={Layers} title="Window backdrop" titleId="backdrop-label">
            {#snippet description()}
              Mica tints the window with your wallpaper colour, Mica Alt more strongly, and Acrylic
              blurs what's behind it. Windows 11 only, and not while Windows shows your accent colour
              on title bars.
            {/snippet}
            {#snippet control()}
              <div class="seg" role="group" aria-labelledby="backdrop-label">
                {#each BACKDROPS as b (b.value)}
                  <button
                    class="seg-btn"
                    class:on={$settings.backdrop === b.value}
                    aria-pressed={$settings.backdrop === b.value}
                    onclick={() => setBackdrop(b.value)}
                  >
                    {b.label}
                  </button>
                {/each}
              </div>
            {/snippet}
          </SettingCard>
        </section>

        <section class="group" aria-labelledby="h-icons">
          <h2 id="h-icons">App icons</h2>
          <SettingCard icon={Image} title="Download app icons">
            {#snippet description()}
              Fetches each app's icon from its website and keeps it on this PC.
              {#if iconStats}
                <span class="nowrap">
                  {iconStats.icons === 0
                    ? 'Nothing cached yet.'
                    : `${iconStats.icons} ${iconStats.icons === 1 ? 'icon' : 'icons'} cached · ${formatBytes(iconStats.bytes)}.`}
                </span>
              {/if}
            {/snippet}
            {#snippet control()}{@render toggleSwitch($settings.downloadIcons, setDownloadIcons, 'Download app icons')}{/snippet}
            <div class="sub-actions">
              <button
                class="btn btn-ghost small"
                onclick={refetchMissingIcons}
                disabled={!$settings.downloadIcons || iconRefetching || clearing}
              >
                <RefreshCw size={14} class={iconRefetching ? 'icon-spin' : ''} />
                {iconRefetching && $iconRefetch.status === 'running' && $iconRefetch.total > 0
                  ? `Downloading ${Math.min($iconRefetch.current + 1, $iconRefetch.total)} of ${$iconRefetch.total}…`
                  : 'Re-download missing'}
              </button>
              <button class="btn btn-ghost small" onclick={clearIcons} disabled={clearing || iconRefetching}>
                <Trash2 size={14} /> Clear cache
              </button>
              {#if $iconRefetch.status === 'done'}<span class="muted small-text">{$iconRefetch.message}</span>{/if}
            </div>
          </SettingCard>
          <SettingCard
            icon={KeyRound}
            title="SteamGridDB API key"
            description="Optional. Fetches icons for Steam and other launcher games (Epic, GOG, EA, Ubisoft, Battle.net)."
          >
            {#snippet control()}
              <input
                class="text-input mono key-input"
                type="password"
                autocomplete="off"
                spellcheck="false"
                placeholder="Paste key…"
                aria-label="SteamGridDB API key"
                value={$settings.steamGridKey}
                onchange={(e) => setSteamGridKey(e.currentTarget.value)}
              />
            {/snippet}
          </SettingCard>
        </section>
      {:else if activeTab === 'sources'}
        <section class="group" aria-labelledby="h-managers">
          <h2 id="h-managers">Package managers</h2>
          <SettingCard
            icon={Star}
            title="Preferred source"
            titleId="pref-label"
            description="Which manager Install uses when an app is available from several."
          >
            {#snippet control()}
              <select
                class="text-input select"
                aria-labelledby="pref-label"
                value={$settings.preferredSource ?? ''}
                onchange={(e) => setPreferredSource((e.currentTarget.value || null) as Source | null)}
              >
                <option value="">No preference</option>
                {#each allManagers as s (s)}
                  <option value={s}>{names[s]}</option>
                {/each}
              </select>
            {/snippet}
          </SettingCard>
          {#each allManagers as s (s)}
            {@const st = statusOf(s)}
            {@const state = managerState(s)}
            <SettingCard icon={Package} title={names[s]}>
              {#snippet description()}
                <span class="pill {state.kind}">{state.label}</span>
                {MANAGER_INFO[s]}
              {/snippet}
              {#snippet control()}
                {#if st && !st.available}
                  <button class="btn" onclick={() => install(s)} disabled={busy === s}>
                    {busy === s ? 'Installing…' : 'Install'}
                  </button>
                {/if}
                {@render toggleSwitch($settings.managers[s] !== false, (on) => setManagerEnabled(s, on), `Use ${names[s]}`)}
              {/snippet}
              {#if st?.available && (s === 'winget' || s === 'scoop')}
                <div class="sub-actions">
                  {#if s === 'winget'}
                    <button
                      class="btn btn-ghost small"
                      disabled={maintBusy !== null}
                      onclick={() => runMaint('Update winget sources', 'winget-src', api.wingetUpdateSources)}
                    >
                      <RefreshCw size={14} class={maintBusy === 'winget-src' ? 'icon-spin' : ''} /> Update sources
                    </button>
                  {:else}
                    <a class="btn btn-ghost small" href="/scoop-buckets"><Package size={14} /> Manage buckets</a>
                    <button
                      class="btn btn-ghost small"
                      disabled={maintBusy !== null}
                      onclick={() => runMaint('Update Scoop', 'scoop-up', api.scoopUpdate)}
                    >
                      <RefreshCw size={14} class={maintBusy === 'scoop-up' ? 'icon-spin' : ''} /> Update Scoop
                    </button>
                    <button
                      class="btn btn-ghost small"
                      disabled={maintBusy !== null}
                      onclick={() => runMaint('Clean up Scoop', 'scoop-clean', api.scoopCleanup)}
                    >
                      <Trash2 size={14} /> Clean up old versions
                    </button>
                  {/if}
                </div>
              {/if}
            </SettingCard>
          {/each}
        </section>

        <section class="group" aria-labelledby="h-catalog">
          <h2 id="h-catalog">App catalog</h2>
          <SettingCard icon={BookOpen} title="Show curated apps">
            {#snippet description()}
              {#if $catalog.status === 'downloading'}
                Downloading Acy's app catalog…
              {:else if $catalog.status === 'error'}
                {$catalog.message}
              {:else if customCat}
                Using your own catalog (below).
              {:else if $settings.showCuratedApps && $catalog.status === 'ready'}
                {$catalog.appCount} apps · catalog v{$catalog.version}
              {:else}
                Hand-picked apps on Discover, downloaded from Acy's GitHub.
              {/if}
            {/snippet}
            {#snippet control()}{@render toggleSwitch($settings.showCuratedApps, toggleCurated, 'Show curated apps')}{/snippet}
            <div class="sub-actions">
              {#if $settings.showCuratedApps && $catalog.status === 'error'}
                <button class="btn btn-ghost small" onclick={() => downloadCatalog()}>
                  <RefreshCw size={14} /> Try again
                </button>
              {:else if catalogPhase === 'available'}
                <button class="btn btn-accent small" onclick={applyCatalog}>Update to v{catalogVersion}</button>
              {:else if !customCat}
                <button
                  class="btn btn-ghost small"
                  onclick={checkCatalog}
                  disabled={catalogPhase === 'checking' || catalogPhase === 'applying'}
                >
                  <RefreshCw size={14} class={catalogPhase === 'checking' || catalogPhase === 'applying' ? 'icon-spin' : ''} />
                  {catalogPhase === 'applying' ? 'Updating…' : 'Check for updates'}
                </button>
              {/if}
              <a class="btn btn-ghost small" href="/curated"><SquarePen size={14} /> Edit your list</a>
              {#if catalogMsg && catalogPhase !== 'available'}<span class="muted small-text">{catalogMsg}</span>{/if}
            </div>
          </SettingCard>

          <details class="setting-expander" open={!!customCat}>
            <summary>
              <span class="ic" aria-hidden="true"><Link size={20} /></span>
              <span class="text">
                <span class="title">Custom catalog</span>
                <span class="desc">
                  {#if customCat}
                    In use: v{customCat.version} · {customCat.appCount} apps
                  {:else}
                    Use your own catalog file or URL instead of Acy's.
                  {/if}
                </span>
              </span>
              <ChevronDown size={18} class="chev" aria-hidden="true" />
            </summary>
            <div class="expander-body">
              {#if customCat}
                <div class="custom-src">
                  {#if customCat.isUrl}<Link size={14} />{:else}<FileText size={14} />{/if}
                  <span class="mono path">{customCat.source}</span>
                </div>
                <div class="sub-actions">
                  <button class="btn small" onclick={refreshCustomCatalog} disabled={customBusy}>
                    {customBusy ? 'Working…' : 'Reload'}
                  </button>
                  <button class="btn btn-ghost small" onclick={removeCustomCatalog} disabled={customBusy}>
                    Stop using it
                  </button>
                </div>
              {:else}
                <div class="custom-row">
                  <button class="btn small" onclick={chooseCustomFile} disabled={customBusy}>
                    <FileText size={14} /> Choose file…
                  </button>
                  <span class="muted small-text">or</span>
                  <input
                    class="text-input mono"
                    type="url"
                    placeholder="https://…/curated.json"
                    aria-label="Catalog URL"
                    autocomplete="off"
                    spellcheck="false"
                    bind:value={customUrl}
                    onkeydown={(e) => e.key === 'Enter' && useCustomUrl()}
                  />
                  <button class="btn small" onclick={useCustomUrl} disabled={customBusy || !customUrl.trim()}>
                    Use URL
                  </button>
                </div>
              {/if}
              {#if customMsg}<p class="muted small-text" role="status">{customMsg}</p>{/if}
            </div>
          </details>
        </section>
      {:else if activeTab === 'updates'}
        <section class="group" aria-labelledby="h-app-updates">
          <h2 id="h-app-updates">Your apps</h2>
          <SettingCard
            toggle
            icon={RefreshCw}
            title="Check for updates on startup"
            description="Look for newer versions of your installed apps when Acy opens (and every 30 minutes)."
          >
            {#snippet control()}{@render toggleSwitch($settings.refreshOnStartup, setRefreshOnStartup)}{/snippet}
          </SettingCard>
          <SettingCard
            toggle
            icon={BellRing}
            title="Notify when updates are found"
            description="A Windows notification when new app updates turn up while Acy is in the background."
          >
            {#snippet control()}{@render toggleSwitch($settings.notifyUpdates, setNotifyUpdates)}{/snippet}
          </SettingCard>
        </section>

        <section class="group" aria-labelledby="h-acy-updates">
          <h2 id="h-acy-updates">Acy</h2>
          <SettingCard icon={Sparkles} title={`Acy v${appVersion || '…'}`}>
            {#snippet description()}
              {#if $updaterPhase === 'available'}
                <span class="pill ok">Update available</span> Version {$updaterVersion} is ready to install.
              {:else if $updaterPhase === 'uptodate'}
                You're on the latest version.
              {:else if $updaterPhase === 'checking'}
                Checking for updates…
              {:else if $updaterPhase === 'downloading'}
                Downloading the update…
              {:else if $updaterPhase === 'error'}
                <span class="err-text">Update check failed: {$updaterError}</span>
              {:else}
                Updates come from Acy's GitHub releases.
              {/if}
            {/snippet}
            {#snippet control()}
              {#if $updaterPhase === 'available'}
                <button class="btn btn-accent" onclick={installUpdate}>Install v{$updaterVersion}</button>
              {:else if $updaterPhase === 'downloading'}
                <button class="btn btn-accent" disabled>Downloading…</button>
              {:else}
                <button class="btn" onclick={checkForUpdate} disabled={$updaterPhase === 'checking'}>
                  <RefreshCw size={14} class={$updaterPhase === 'checking' ? 'icon-spin' : ''} />
                  Check for updates
                </button>
              {/if}
            {/snippet}
          </SettingCard>
          <SettingCard
            toggle
            icon={Bell}
            title="Check for Acy updates automatically"
            description="Look for a new version of Acy in the background and tell you when there is one."
          >
            {#snippet control()}{@render toggleSwitch($settings.autoCheckUpdates, onAutoCheckToggle)}{/snippet}
          </SettingCard>
        </section>
      {:else if activeTab === 'shortcuts'}
        <section class="group" aria-labelledby="h-keys">
          <h2 id="h-keys">Keyboard</h2>
          <p class="note muted">Press <kbd class="key">?</kbd> anywhere to bring this list up.</p>
          <div class="panel">
            <ShortcutList />
          </div>
        </section>
      {:else if activeTab === 'about'}
        <section class="group" aria-label="About Acy">
          <div class="identity">
            <img src="/acy-icon.png" alt="" width="56" height="56" />
            <div class="identity-text">
              <span class="identity-name">Acy</span>
              <span class="muted">Version <span class="mono">{appVersion || '…'}</span> · MIT licence</span>
            </div>
            <button class="btn" onclick={() => openUrl('https://github.com/0x89-y/Acy')}>
              <ExternalLink size={14} /> GitHub
            </button>
          </div>
        </section>

        <section class="group" aria-labelledby="h-new">
          <h2 id="h-new">What's new</h2>
          {#each CHANGELOG.slice(0, 1) as rel (rel.version)}
            <div class="panel release">
              <div class="rel-head">
                <ScrollText size={18} aria-hidden="true" />
                <span class="rel-ver">Version {rel.version}</span>
                <span class="mono muted rel-date">{rel.date}</span>
              </div>
              <ul class="rel-list">
                {#each rel.changes as c (c)}
                  <li>{c}</li>
                {/each}
              </ul>
              <a class="btn btn-ghost small" href="/changelog">View full changelog</a>
            </div>
          {/each}
        </section>
      {/if}
    </div>
  </div>
</div>

<style>
  .page {
    max-width: 880px;
    padding: 0 24px 32px;
    display: flex;
    flex-direction: column;
    gap: 22px;
  }
  .group {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .group h2 {
    margin: 0 0 6px;
    font-size: var(--fs-base);
    font-weight: 600;
  }
  .note {
    margin: 0 0 8px;
    font-size: var(--fs-sm);
  }
  .nowrap {
    white-space: nowrap;
  }
  .path {
    font-size: var(--fs-xs);
    color: var(--text);
    overflow-wrap: anywhere;
  }
  .small {
    padding: 5px 11px;
    font-size: var(--fs-sm);
  }
  .small-text {
    font-size: var(--fs-sm);
  }
  .err-text {
    color: var(--danger);
  }
  .sub-actions {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px 8px;
    margin-left: -11px;
  }
  .sub-actions .btn-ghost {
    color: var(--text);
  }
  .danger-outline {
    color: var(--danger);
    border-color: color-mix(in srgb, var(--danger) 45%, var(--border-strong));
  }
  .danger-outline:hover:not(:disabled) {
    background: var(--danger-fill);
    border-color: var(--danger-fill);
    color: #fff;
  }
  .key-input {
    width: 220px;
  }
  .select {
    width: auto;
    min-width: 180px;
  }

  /* Status pill (package managers, Acy update) */
  .pill {
    display: inline-block;
    margin-right: 6px;
    padding: 0 8px;
    border-radius: var(--radius-pill);
    font-size: var(--fs-2xs);
    font-weight: 600;
    line-height: 1.7;
    vertical-align: 1px;
  }
  .pill.ok {
    color: var(--success);
    background: color-mix(in srgb, var(--success) 14%, transparent);
  }
  .pill.warn {
    color: var(--warning);
    background: color-mix(in srgb, var(--warning) 16%, transparent);
  }
  .pill.off {
    color: var(--text-muted);
    background: color-mix(in srgb, var(--text) 8%, transparent);
  }

  /* Segmented control */
  .seg {
    display: inline-flex;
    gap: 2px;
    padding: 2px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }
  .seg-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-muted);
    font-size: var(--fs-md);
    font-weight: 500;
    white-space: nowrap;
  }
  .seg-btn:hover {
    color: var(--text);
    background: var(--surface-hover);
  }
  .seg-btn.on {
    background: var(--accent-fill);
    color: var(--accent-contrast);
  }

  /* Accent swatches */
  .accents {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
  }
  .swatch {
    width: 26px;
    height: 26px;
    padding: 0;
    border: none;
    border-radius: 50%;
    background: var(--sw);
    cursor: pointer;
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
      0 0 0 2px var(--surface-solid),
      0 0 0 4px var(--sw);
  }
  .swatch.aurora.on {
    box-shadow:
      0 0 0 2px var(--surface-solid),
      0 0 0 4px #9b7bd0;
  }

  /*
   * Toggle switch, Windows style: an outlined track when off (a white knob on
   * light grey fell short of 3:1), accent-filled when on.
   */
  .switch {
    position: relative;
    display: inline-block;
    width: 40px;
    height: 20px;
    flex-shrink: 0;
  }
  .switch input {
    position: absolute;
    opacity: 0;
    width: 0;
    height: 0;
  }
  .slider {
    position: absolute;
    inset: 0;
    cursor: pointer;
    border: 1.5px solid var(--text-muted);
    border-radius: var(--radius-pill);
    transition: background 0.15s, border-color 0.15s;
  }
  .slider::before {
    content: '';
    position: absolute;
    width: 12px;
    height: 12px;
    left: 2.5px;
    top: 2.5px;
    border-radius: 50%;
    background: var(--text-muted);
    transition: transform 0.15s, background 0.15s;
  }
  .switch input:checked + .slider {
    background: var(--accent-fill);
    border-color: transparent;
  }
  .switch input:checked + .slider::before {
    transform: translateX(20px);
    background: var(--accent-contrast);
  }
  .switch input:focus-visible + .slider {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  /* Expander card (custom catalog) */
  .setting-expander {
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface-2);
  }
  .setting-expander summary {
    display: flex;
    align-items: center;
    gap: 16px;
    min-height: 64px;
    padding: 12px 16px;
    cursor: pointer;
    list-style: none;
    border-radius: var(--radius);
  }
  .setting-expander summary::-webkit-details-marker {
    display: none;
  }
  .setting-expander summary:hover {
    background: var(--surface-hover);
  }
  .setting-expander .ic {
    display: inline-flex;
    flex-shrink: 0;
  }
  .setting-expander .text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .setting-expander .title {
    font-size: var(--fs-base);
    font-weight: 500;
  }
  .setting-expander .desc {
    font-size: var(--fs-sm);
    color: var(--text-muted);
  }
  .setting-expander :global(.chev) {
    flex-shrink: 0;
    color: var(--text-muted);
    transition: transform 0.15s;
  }
  .setting-expander[open] :global(.chev) {
    transform: rotate(180deg);
  }
  .expander-body {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 12px 16px 14px 52px;
    border-top: 1px solid var(--border);
  }
  .expander-body p {
    margin: 0;
  }
  .custom-row {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
  }
  .custom-row .text-input {
    flex: 1;
    min-width: 200px;
  }
  .custom-src {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }

  /* Plain panels (shortcuts, release notes) */
  .panel {
    padding: 12px 16px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface-2);
  }
  .key {
    font-family: var(--font-mono);
    font-size: var(--fs-xs);
    padding: 1px 6px;
    border: 1px solid var(--border-strong);
    border-radius: 5px;
  }

  /* About */
  .identity {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 18px 20px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface-2);
  }
  .identity img {
    border-radius: 12px;
  }
  .identity-text {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-size: var(--fs-sm);
  }
  .identity-name {
    font-size: var(--fs-xl);
    font-weight: 600;
  }
  .release {
    display: flex;
    flex-direction: column;
    gap: 8px;
    align-items: flex-start;
  }
  .rel-head {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .rel-ver {
    font-weight: 600;
  }
  .rel-date {
    font-size: var(--fs-xs);
  }
  .rel-list {
    margin: 0;
    padding-left: 20px;
    font-size: var(--fs-md);
    color: var(--text-muted);
    line-height: 1.55;
  }
  .rel-list li + li {
    margin-top: 3px;
  }

  :global(.icon-spin) {
    animation: icon-spin 0.9s linear infinite;
  }
  @keyframes icon-spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>

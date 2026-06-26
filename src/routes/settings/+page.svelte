<script lang="ts">
  import { onMount } from 'svelte';
  import { getVersion } from '@tauri-apps/api/app';
  import { Sun, Moon, Monitor } from '@lucide/svelte';
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
    setRefreshOnStartup,
    restartSetup,
    ACCENTS,
    type ThemeMode
  } from '$lib/stores/settings';
  import { managers, loadManagers } from '$lib/stores/managers';
  import { clearIconCache } from '$lib/stores/icons';
  import { enqueue } from '$lib/stores/ops';
  import { activity, clearActivity, type ActivityAction } from '$lib/stores/activity';
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
  let showAllChanges = $state(false);
  let showAllActivity = $state(false);

  let shownReleases = $derived(showAllChanges ? CHANGELOG : CHANGELOG.slice(0, 1));
  let olderCount = $derived(CHANGELOG.length - 1);

  /** Number of activity rows shown before the "show all" toggle. */
  const ACTIVITY_PREVIEW = 6;
  let shownActivity = $derived(
    showAllActivity ? $activity : $activity.slice(0, ACTIVITY_PREVIEW)
  );

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
</script>

<h1>Settings</h1>

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
  <h2>Package managers</h2>
  <p class="muted hint">
    Turn managers on or off. A disabled manager is skipped in search, installed apps, and updates.
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
      For apps offered by several managers, the Install button uses this one when available; you can
      still pick another from its menu.
    </p>
  </div>
  <div class="list">
    {#each allManagers as s (s)}
      {@const st = statusOf(s)}
      {@const isLocal = s === 'local'}
      <div class="row card">
        <div class="info">
          <span class="name">{names[s]}</span>
          <span class="state mono" class:ok={isLocal || st?.available} class:off={!isLocal && !st?.available}>
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

<section class="group">
  <h2>App icons</h2>
  <label class="toggle-row card">
    <span class="toggle-text">
      <span class="toggle-title">Download &amp; cache app icons</span>
      <span class="toggle-sub muted">
        Off by default. Icons are fetched from each app's website as you browse and stored on
        disk, so they load instantly next time. Apps without a known website keep the lettered tile.
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

<section class="group">
  <h2>Installs</h2>
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
        Refresh installed apps and available updates automatically when Acy starts. Turn off for a
        faster launch; you can still refresh manually on the Installed page.
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
</section>

<section class="group">
  <h2>Tray &amp; notifications</h2>
  <label class="toggle-row card">
    <span class="toggle-text">
      <span class="toggle-title">Close to tray instead of quitting</span>
      <span class="toggle-sub muted">
        Acy keeps running in the system tray after you close the window and checks for updates in
        the background. Quit from the tray icon's menu.
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
  <h2>Curated catalog</h2>
  <p class="muted hint">Edit the categories and apps shown on the Discover home page.</p>
  <a class="btn" href="/curated">Open catalog editor</a>
</section>

<section class="group">
  <h2>First-run setup</h2>
  <p class="muted hint">Show the welcome setup screen again to reconfigure from scratch.</p>
  <button class="btn" onclick={restartSetup}>Run setup again</button>
</section>

<section class="group">
  <h2>Activity</h2>
  {#if $activity.length === 0}
    <p class="muted hint">Your installs, updates, and removals will show up here.</p>
  {:else}
    <div class="log">
      {#each shownActivity as a (a.id)}
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
      {#if $activity.length > ACTIVITY_PREVIEW}
        <button class="btn btn-ghost" onclick={() => (showAllActivity = !showAllActivity)}>
          {showAllActivity ? 'Show less' : `Show all ${$activity.length}`}
        </button>
      {/if}
      <button class="btn btn-ghost" onclick={clearActivity}>Clear activity</button>
    </div>
  {/if}
</section>

<section class="group">
  <h2>What's new</h2>
  <div class="log">
    {#each shownReleases as rel (rel.version)}
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
  {#if olderCount > 0}
    <button class="btn btn-ghost" onclick={() => (showAllChanges = !showAllChanges)}>
      {showAllChanges ? 'Show less' : `Show ${olderCount} older release${olderCount === 1 ? '' : 's'}`}
    </button>
  {/if}
</section>

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
  .group {
    margin: 0 auto 32px;
    max-width: 600px;
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
    margin-bottom: 16px;
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
    margin: 8px 0 0;
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

  .list {
    display: flex;
    flex-direction: column;
    gap: 8px;
    text-align: left;
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
</style>

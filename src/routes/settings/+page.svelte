<script lang="ts">
  import { onMount } from 'svelte';
  import { getVersion } from '@tauri-apps/api/app';
  import { Sun, Moon, Monitor } from '@lucide/svelte';
  import {
    settings,
    setThemeMode,
    setAccent,
    setManagerEnabled,
    setShowOutput,
    setDownloadIcons,
    ACCENTS,
    type ThemeMode
  } from '$lib/stores/settings';
  import { managers, loadManagers } from '$lib/stores/managers';
  import { clearIconCache } from '$lib/stores/icons';
  import { enqueue } from '$lib/stores/ops';
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
    choco: 'Chocolatey'
  };
  const allManagers: Source[] = ['winget', 'scoop', 'choco'];

  let busy = $state<Source | null>(null);
  let clearing = $state(false);
  let appVersion = $state('');

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
  <div class="list">
    {#each allManagers as s (s)}
      {@const st = statusOf(s)}
      <div class="row card">
        <div class="info">
          <span class="name">{names[s]}</span>
          <span class="state mono" class:ok={st?.available} class:off={!st?.available}>
            {st?.available ? 'available' : 'not installed'}
          </span>
        </div>
        {#if st && !st.available}
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
  <label class="opt">
    <input
      type="checkbox"
      checked={$settings.downloadIcons}
      onchange={(e) => setDownloadIcons(e.currentTarget.checked)}
    />
    <span>Download app icons from the web and cache them</span>
  </label>
  <p class="muted hint">
    Off by default. When on, icons are fetched from each app's website as you browse and
    stored on disk, so they load instantly next time. Apps without a known website keep the
    lettered tile.
  </p>
  <button class="btn" onclick={clearIcons} disabled={clearing}>
    {clearing ? 'Clearing…' : 'Clear icon cache'}
  </button>
</section>

<section class="group">
  <h2>Installs</h2>
  <label class="opt">
    <input
      type="checkbox"
      checked={$settings.showOutput}
      onchange={(e) => setShowOutput(e.currentTarget.checked)}
    />
    <span>Show command output by default while installing</span>
  </label>
</section>

<section class="group">
  <h2>About</h2>
  <p class="about muted">Acy <span class="mono">v{appVersion || '…'}</span></p>
</section>

<style>
  .about {
    font-size: 0.9rem;
  }
  h1 {
    margin-bottom: 24px;
  }
  .group {
    margin-bottom: 32px;
    max-width: 640px;
  }
  .group h2 {
    font-size: 1.05rem;
    margin-bottom: 12px;
  }
  .hint {
    font-size: 0.86rem;
    margin: -4px 0 12px;
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

  .opt {
    display: flex;
    align-items: center;
    gap: 10px;
    cursor: pointer;
    font-size: 0.92rem;
  }
  .opt input {
    width: 16px;
    height: 16px;
    accent-color: var(--accent);
  }
</style>

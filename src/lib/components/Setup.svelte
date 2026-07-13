<script lang="ts">
  import { onMount } from 'svelte';
  import { Sun, Moon, Monitor } from '@lucide/svelte';
  import {
    settings,
    setThemeMode,
    setAccent,
    setManagerEnabled,
    setDownloadIcons,
    completeSetup,
    ACCENTS,
    type ThemeMode
  } from '$lib/stores/settings';
  import { managers, loadManagers } from '$lib/stores/managers';
  import { enqueue } from '$lib/stores/ops';
  import WindowControls from './WindowControls.svelte';
  import * as api from '$lib/api';
  import type { Source, Manager } from '$lib/types';

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
  const allManagers: Manager[] = ['winget', 'scoop', 'choco', 'msstore'];

  // One-line descriptions, matching the Sources tab in Settings.
  const MANAGER_INFO: Record<Manager, string> = {
    winget: "Windows Package Manager — Microsoft's built-in catalog",
    scoop: 'Portable apps and developer tools',
    choco: 'Chocolatey — large community catalog',
    msstore: 'Microsoft Store apps'
  };

  let busy = $state<Source | null>(null);

  onMount(() => {
    loadManagers();
  });

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

<div class="shell">
  <div class="titlebar" data-tauri-drag-region>
    <span class="logo mono" data-tauri-drag-region>acy</span>
    <div class="spacer" data-tauri-drag-region></div>
    <WindowControls />
  </div>

  <div class="browse-panel">
    <!-- Welcome / identity sidebar with the primary action, like an app's
         detail page: brand up top, "Get started" pinned at the bottom. -->
    <aside class="side">
      <div class="ident">
        <div class="mark mono">acy</div>
        <p class="lede">A quick setup before you start.</p>
        <p class="sub muted">You can change all of this later in Settings.</p>
      </div>
      <div class="grow"></div>
      <button class="btn btn-accent start" onclick={completeSetup}>Get started</button>
    </aside>

    <div class="panes">
      <section class="group">
        <h2>Appearance</h2>
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
            </div>
          </div>
        </div>
      </section>

      <section class="group">
        <h2>Package managers</h2>
        <div class="mgr-list">
          {#each allManagers as s (s)}
            {@const st = statusOf(s)}
            <div class="mgr-row" class:is-on={$settings.managers[s] !== false}>
              <div class="mgr-meta">
                <span class="mgr-name">{names[s]}</span>
                <span class="mgr-desc muted">{MANAGER_INFO[s]}</span>
              </div>
              <div class="mgr-actions">
                {#if st && !st.available}
                  <button class="btn mgr-btn" onclick={() => install(s)} disabled={busy === s}>
                    {busy === s ? 'Working…' : 'Install'}
                  </button>
                {:else}
                  <span class="mgr-state mono" class:ok={st?.available} class:off={!st?.available}>
                    {st?.available ? 'available' : 'not installed'}
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

      <section class="group">
        <h2>App icons</h2>
        <div class="opt-list">
          <label class="opt-row">
            <span class="opt-label">Download &amp; cache app icons from the web</span>
            <span class="switch">
              <input
                type="checkbox"
                checked={$settings.downloadIcons}
                onchange={(e) => setDownloadIcons(e.currentTarget.checked)}
              />
              <span class="slider"></span>
            </span>
          </label>
        </div>
      </section>
    </div>
  </div>
</div>

<style>
  .shell {
    position: fixed;
    inset: 0;
    z-index: 100;
    display: flex;
    flex-direction: column;
    background: var(--surface);
  }
  /* Slim title bar, matching the app's Nav. */
  .titlebar {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 10px;
    height: 32px;
    padding: 0 4px 0 14px;
    border-bottom: 1px solid var(--border);
    background: var(--surface);
  }
  .titlebar .logo {
    font-size: 1.02rem;
    font-weight: 600;
    letter-spacing: 0.02em;
  }
  .titlebar .logo::before {
    content: '0x';
    color: var(--accent);
  }
  .titlebar .spacer {
    flex: 1;
    align-self: stretch;
  }

  /* Full-bleed master-detail panel. */
  .browse-panel {
    flex: 1;
    min-height: 0;
    display: flex;
    align-items: stretch;
    overflow: hidden;
  }
  .side {
    flex: 0 0 260px;
    display: flex;
    flex-direction: column;
    overflow-y: auto;
    padding: 28px 24px 24px;
    border-right: 1px solid var(--border);
    background: var(--surface-2);
  }
  .ident {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .mark {
    font-size: 2rem;
    font-weight: 600;
    letter-spacing: 0.02em;
    line-height: 1;
    margin-bottom: 10px;
  }
  .mark::before {
    content: '0x';
    color: var(--accent);
  }
  .lede {
    margin: 0;
    font-size: 0.98rem;
    font-weight: 500;
  }
  .sub {
    margin: 0;
    font-size: 0.86rem;
    line-height: 1.45;
  }
  .grow {
    flex: 1;
    min-height: 24px;
  }
  .start {
    width: 100%;
    justify-content: center;
    padding: 11px 16px;
    font-size: 0.95rem;
    font-weight: 600;
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
    background: var(--surface);
  }

  .group {
    margin: 0;
  }
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

  /* Divided rows, inset by the pad — no bordered boxes (shared with Settings). */
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

  .accents {
    display: flex;
    gap: 12px;
    flex-wrap: wrap;
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
  .swatch.on {
    box-shadow:
      0 0 0 2px var(--surface),
      0 0 0 4px var(--sw);
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

  @media (max-width: 720px) {
    .browse-panel {
      flex-direction: column;
    }
    .side {
      flex: none;
      border-right: none;
      border-bottom: 1px solid var(--border);
    }
    .grow {
      min-height: 16px;
    }
  }
</style>

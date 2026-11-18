<script lang="ts">
  import { onMount, tick } from 'svelte';
  import {
    Sun,
    Moon,
    Monitor,
    Check,
    Package,
    SunMoon,
    Palette,
    Layers,
    BookOpen,
    Image,
    TriangleAlert,
    ArrowLeft,
    ArrowRight
  } from '@lucide/svelte';
  import {
    settings,
    setThemeMode,
    setAccent,
    setBackdrop,
    setManagerEnabled,
    setDownloadIcons,
    setShowCuratedApps,
    completeSetup,
    ACCENTS,
    BACKDROPS,
    type ThemeMode
  } from '$lib/stores/settings';
  import { managers, loadManagers } from '$lib/stores/managers';
  import { catalog, downloadCatalog, loadCatalogStatus } from '$lib/stores/curated';
  import { enqueue } from '$lib/stores/ops';
  import * as api from '$lib/api';
  import type { Source, Manager } from '$lib/types';
  import SettingCard from './SettingCard.svelte';
  import QueueTray from './QueueTray.svelte';

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
  const allManagers: Manager[] = ['winget', 'scoop', 'choco', 'msstore'];
  const MANAGER_INFO: Record<Manager, string> = {
    winget: "Windows Package Manager, Microsoft's built-in catalog.",
    scoop: 'Portable apps and developer tools.',
    choco: 'Chocolatey, a large community catalog.',
    msstore: 'Apps from the Microsoft Store.'
  };

  const STEPS = [
    {
      title: 'Package managers',
      intro: 'Acy installs apps through the package managers on this PC. Turn on the ones you want it to use.'
    },
    {
      title: 'Make it yours',
      intro: 'Pick a theme and colour. Changes apply right away.'
    },
    {
      title: 'Apps & icons',
      intro: 'Choose what Acy downloads to show apps on Discover.'
    }
  ];

  let step = $state(0);
  let heading = $state<HTMLHeadingElement | null>(null);
  let busy = $state<Source | null>(null);

  onMount(() => {
    loadManagers();
    if ($settings.showCuratedApps) downloadCatalog();
    else loadCatalogStatus();
  });

  // Move focus to the new step's heading so screen readers announce it.
  async function go(to: number) {
    step = to;
    await tick();
    heading?.focus();
  }

  function toggleCurated(on: boolean) {
    setShowCuratedApps(on);
    if (on) downloadCatalog();
    else loadCatalogStatus();
  }

  function statusOf(source: Source) {
    return $managers.find((m) => m.source === source);
  }

  function managerState(source: Source): { label: string; kind: 'ok' | 'warn' | 'off' } {
    const st = statusOf(source);
    if (!st) return { label: 'Checking…', kind: 'off' };
    if (!st.available) return { label: 'Not installed', kind: 'off' };
    if (st.needsSetup) return { label: 'Needs setup', kind: 'warn' };
    return { label: 'Installed', kind: 'ok' };
  }

  let noneUsable = $derived(
    $managers.length > 0 &&
      !$managers.some((m) => m.available && $settings.managers[m.source] !== false)
  );

  async function install(source: Source) {
    busy = source;
    await enqueue(`Set up ${names[source]}`, (opId) => api.bootstrapManager(source, opId));
    busy = null;
    loadManagers(true);
  }
</script>

{#snippet toggleSwitch(checked: boolean, onchange: (on: boolean) => void, label?: string)}
  <span class="switch">
    <input type="checkbox" {checked} aria-label={label} onchange={(e) => onchange(e.currentTarget.checked)} />
    <span class="slider" aria-hidden="true"></span>
  </span>
{/snippet}

<div class="setup">
  <aside class="intro">
    <div class="brand">
      <img src="/acy-icon.png" alt="" width="48" height="48" />
      <div>
        <p class="welcome">Welcome to Acy</p>
        <p class="tagline">One place to find, install and update your apps.</p>
      </div>
    </div>

    <ol class="steps" aria-label="Setup steps">
      {#each STEPS as s, i (s.title)}
        <li>
          <button
            class="step"
            class:current={i === step}
            class:done={i < step}
            aria-current={i === step ? 'step' : undefined}
            onclick={() => go(i)}
          >
            <span class="dot" aria-hidden="true">
              {#if i < step}<Check size={13} />{:else}{i + 1}{/if}
            </span>
            {s.title}
          </button>
        </li>
      {/each}
    </ol>

    <p class="later">You can change all of this later in Settings.</p>
  </aside>

  <div class="content">
    <div class="scroll">
      <div class="page">
        <header class="head">
          <p class="count">Step {step + 1} of {STEPS.length}</p>
          <h1 tabindex="-1" bind:this={heading}>{STEPS[step].title}</h1>
          <p class="sub">{STEPS[step].intro}</p>
        </header>

        {#if step === 0}
          <div class="cards">
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
                    <button class="btn" onclick={() => install(s)} disabled={busy !== null}>
                      {busy === s ? 'Installing…' : 'Install'}
                    </button>
                  {/if}
                  {@render toggleSwitch($settings.managers[s] !== false, (on) => setManagerEnabled(s, on), `Use ${names[s]}`)}
                {/snippet}
              </SettingCard>
            {/each}
          </div>
          {#if noneUsable}
            <p class="warn" role="status">
              <TriangleAlert size={16} aria-hidden="true" />
              None of the package managers you turned on are installed. Install winget (or another
              one) to search and install apps.
            </p>
          {/if}
        {:else if step === 1}
          <div class="cards">
            <SettingCard icon={SunMoon} title="Theme" titleId="setup-theme" description="Follow Windows, or always use light or dark.">
              {#snippet control()}
                <div class="seg" role="group" aria-labelledby="setup-theme">
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
                      style="--sw:{a.color}"
                      onclick={() => setAccent(a.name)}
                      title={a.label}
                      aria-label={a.label}
                      aria-pressed={$settings.accent === a.name}
                    ></button>
                  {/each}
                </div>
              {/snippet}
            </SettingCard>
            <SettingCard
              icon={Layers}
              title="Window backdrop"
              titleId="setup-backdrop"
              description="Mica tints the window with your wallpaper colour, Acrylic blurs what's behind it. Windows 11 only."
            >
              {#snippet control()}
                <div class="seg" role="group" aria-labelledby="setup-backdrop">
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
          </div>
        {:else}
          <div class="cards">
            <SettingCard toggle icon={BookOpen} title="Show curated apps">
              {#snippet description()}
                {#if $catalog.status === 'downloading'}
                  Downloading Acy's app catalog…
                {:else if $catalog.status === 'error'}
                  {$catalog.message}
                {:else if $settings.showCuratedApps && $catalog.status === 'ready'}
                  Ready: {$catalog.appCount} hand-picked apps, plus collections like "New PC essentials".
                {:else}
                  Hand-picked apps and collections on Discover, downloaded from Acy's GitHub.
                {/if}
              {/snippet}
              {#snippet control()}{@render toggleSwitch($settings.showCuratedApps, toggleCurated)}{/snippet}
            </SettingCard>
            {#if $settings.showCuratedApps && $catalog.status === 'error'}
              <div class="retry">
                <button class="btn" onclick={() => downloadCatalog()}>Try again</button>
              </div>
            {/if}
            <SettingCard
              toggle
              icon={Image}
              title="Download app icons"
              description="Fetches each app's icon from its own website and keeps it on this PC. Off: apps show a coloured letter instead."
            >
              {#snippet control()}{@render toggleSwitch($settings.downloadIcons, setDownloadIcons)}{/snippet}
            </SettingCard>
          </div>
        {/if}
      </div>
    </div>

    <footer class="foot">
      <button class="btn btn-ghost skip" onclick={completeSetup}>Skip setup</button>
      <div class="spacer"></div>
      {#if step > 0}
        <button class="btn" onclick={() => go(step - 1)}><ArrowLeft size={15} /> Back</button>
      {/if}
      {#if step < STEPS.length - 1}
        <button class="btn btn-tinted" onclick={() => go(step + 1)}>Next <ArrowRight size={15} /></button>
      {:else}
        <button class="btn btn-tinted" onclick={completeSetup}>Get started <ArrowRight size={15} /></button>
      {/if}
    </footer>
    <QueueTray />
  </div>
</div>

<style>
  /* Same shell as the main window: a column on the backdrop, content on a panel. */
  .setup {
    position: fixed;
    inset: 0;
    z-index: 100;
    display: flex;
    background: var(--shell);
  }
  .intro {
    width: 280px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    gap: 28px;
    padding: 28px 20px 24px;
  }
  .brand {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .brand img {
    border-radius: 12px;
  }
  .welcome {
    margin: 0;
    font-size: var(--fs-xl);
    font-weight: 600;
  }
  .tagline {
    margin: 4px 0 0;
    font-size: var(--fs-md);
    color: var(--text-muted);
  }
  .steps {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .step {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 12px;
    min-height: 40px;
    padding: 0 10px;
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-muted);
    font-size: var(--fs-base);
    font-weight: 500;
    text-align: left;
  }
  .step:hover {
    background: var(--shell-hover);
    color: var(--text);
  }
  .step.current {
    background: var(--shell-active);
    color: var(--text);
  }
  .dot {
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    border: 1.5px solid currentColor;
    border-radius: 50%;
    font-size: var(--fs-xs);
    font-weight: 600;
  }
  .step.current .dot,
  .step.done .dot {
    border-color: color-mix(in srgb, var(--accent) 70%, var(--text));
    background: color-mix(in srgb, var(--accent) 20%, transparent);
    color: color-mix(in srgb, var(--accent) 70%, var(--text));
  }
  .later {
    margin: auto 0 0;
    font-size: var(--fs-sm);
    color: var(--text-muted);
  }

  .content {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    background: var(--panel);
    border-top: 1px solid var(--panel-border);
    border-left: 1px solid var(--panel-border);
    border-top-left-radius: 8px;
  }
  .scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
  }
  .page {
    max-width: 760px;
    padding: 32px 32px 24px;
  }
  .head {
    margin-bottom: 20px;
  }
  .count {
    margin: 0 0 4px;
    font-size: var(--fs-sm);
    font-weight: 600;
    color: color-mix(in srgb, var(--accent) 70%, var(--text));
  }
  .head h1 {
    font-size: 1.6rem;
    outline: none;
  }
  .sub {
    margin: 6px 0 0;
    max-width: 60ch;
    color: var(--text-muted);
  }
  .cards {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .warn {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    margin: 14px 0 0;
    padding: 10px 14px;
    border-radius: var(--radius);
    background: color-mix(in srgb, var(--warning) 14%, transparent);
    font-size: var(--fs-md);
  }
  .warn :global(svg) {
    flex-shrink: 0;
    margin-top: 2px;
    color: var(--warning);
  }
  .retry {
    padding: 4px 0 8px 52px;
  }
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
  .foot {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 14px 32px 18px;
    border-top: 1px solid var(--border);
  }
  .spacer {
    flex: 1;
  }
  .skip {
    color: var(--text-muted);
  }
</style>

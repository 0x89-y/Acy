<script lang="ts">
  import '@fontsource/ibm-plex-sans/400.css';
  import '@fontsource/ibm-plex-sans/500.css';
  import '@fontsource/ibm-plex-sans/600.css';
  import '@fontsource/ibm-plex-mono/400.css';
  import '@fontsource/ibm-plex-mono/500.css';
  import '$lib/styles/app.css';

  import { onMount } from 'svelte';
  import { get } from 'svelte/store';
  import Nav from '$lib/components/Nav.svelte';
  import OpStack from '$lib/components/OpStack.svelte';
  import ResizeHandles from '$lib/components/ResizeHandles.svelte';
  import Setup from '$lib/components/Setup.svelte';
  import { loadManagers } from '$lib/stores/managers';
  import { loadUpdates } from '$lib/stores/library';
  import { settings } from '$lib/stores/settings';
  import { initTray } from '$lib/stores/tray';
  import { initAppUpdater } from '$lib/stores/updater';
  import '$lib/stores/theme'; // applies data-theme reactively from settings

  let { children } = $props();

  /** How often to re-check for available updates while the app is open. */
  const UPDATE_POLL_MS = 30 * 60 * 1000;

  onMount(() => {
    initTray();
    initAppUpdater();
    // Detect managers first so the update check runs against the right sources,
    // then keep the nav badge current with a periodic background refresh. The
    // initial check is skipped when the user turns off startup refresh.
    (async () => {
      await loadManagers();
      if (get(settings).refreshOnStartup) loadUpdates();
    })();
    const timer = setInterval(() => loadUpdates(true), UPDATE_POLL_MS);
    return () => clearInterval(timer);
  });
</script>

{#if !$settings.setupComplete}
  <Setup />
{:else}
  <Nav />
  <main class="page">
    {@render children()}
  </main>
{/if}
<OpStack />
<ResizeHandles />

<style>
  .page {
    max-width: 1180px;
    margin: 0 auto;
    padding: 26px 24px 90px;
  }
</style>

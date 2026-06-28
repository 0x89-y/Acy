<script lang="ts">
  import '@fontsource/ibm-plex-sans/400.css';
  import '@fontsource/ibm-plex-sans/500.css';
  import '@fontsource/ibm-plex-sans/600.css';
  import '@fontsource/ibm-plex-mono/400.css';
  import '@fontsource/ibm-plex-mono/500.css';
  import '$lib/styles/app.css';

  import { onMount } from 'svelte';
  import { get } from 'svelte/store';
  import { goto } from '$app/navigation';
  import Nav from '$lib/components/Nav.svelte';
  import OpStack from '$lib/components/OpStack.svelte';
  import AppUpdateToast from '$lib/components/AppUpdateToast.svelte';
  import ContextMenu from '$lib/components/ContextMenu.svelte';
  import CloseToTrayPrompt from '$lib/components/CloseToTrayPrompt.svelte';
  import ShortcutsHelp, { toggleShortcuts } from '$lib/components/ShortcutsHelp.svelte';
  import ResizeHandles from '$lib/components/ResizeHandles.svelte';
  import Setup from '$lib/components/Setup.svelte';
  import { loadManagers } from '$lib/stores/managers';
  import { loadUpdates } from '$lib/stores/library';
  import { settings } from '$lib/stores/settings';
  import { initTray } from '$lib/stores/tray';
  import { initAppUpdater } from '$lib/stores/updater';
  import '$lib/stores/theme';

  let { children } = $props();

  const UPDATE_POLL_MS = 30 * 60 * 1000;

  function isTyping(e: KeyboardEvent) {
    const t = e.target as HTMLElement | null;
    return t?.tagName === 'INPUT' || t?.tagName === 'TEXTAREA' || t?.isContentEditable === true;
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === '?' && !isTyping(e)) {
      e.preventDefault();
      toggleShortcuts();
      return;
    }
    if (!(e.ctrlKey || e.metaKey) || e.altKey || e.shiftKey) return;
    const dest = e.key === '1' ? '/' : e.key === '2' ? '/installed' : e.key === '3' ? '/settings' : null;
    if (dest) {
      e.preventDefault();
      goto(dest);
    }
  }

  onMount(() => {
    initTray();
    initAppUpdater();
    window.addEventListener('keydown', onKey);
    (async () => {
      await loadManagers();
      if (get(settings).refreshOnStartup) loadUpdates();
    })();
    const timer = setInterval(() => loadUpdates(true), UPDATE_POLL_MS);
    return () => {
      clearInterval(timer);
      window.removeEventListener('keydown', onKey);
    };
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
<AppUpdateToast />
<ContextMenu />
<CloseToTrayPrompt />
<ShortcutsHelp />
<ResizeHandles />

<style>
  .page {
    max-width: 1180px;
    margin: 0 auto;
    padding: 26px 24px 90px;
  }
</style>

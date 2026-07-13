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
  import { page } from '$app/stores';
  import Nav from '$lib/components/Nav.svelte';
  import OpStack from '$lib/components/OpStack.svelte';
  import AppUpdateToast from '$lib/components/AppUpdateToast.svelte';
  import ContextMenu from '$lib/components/ContextMenu.svelte';
  import CloseToTrayPrompt from '$lib/components/CloseToTrayPrompt.svelte';
  import ShortcutsHelp, { toggleShortcuts } from '$lib/components/ShortcutsHelp.svelte';
  import ResizeHandles from '$lib/components/ResizeHandles.svelte';
  import WingetBusy from '$lib/components/WingetBusy.svelte';
  import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';
  import Setup from '$lib/components/Setup.svelte';
  import { loadManagers } from '$lib/stores/managers';
  import { loadUpdates } from '$lib/stores/library';
  import { browseView } from '$lib/stores/discover';
  import { settings } from '$lib/stores/settings';
  import { initTray } from '$lib/stores/tray';
  import { initAppUpdater } from '$lib/stores/updater';
  import '$lib/stores/theme';

  let { children } = $props();

  let fullBleed = $derived(
    ['/', '/settings', '/changelog', '/curated'].includes($page.url.pathname) ||
      $page.url.pathname.startsWith('/app/')
  );

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
    if (e.key === '1') {
      e.preventDefault();
      browseView.set('discover');
      goto('/');
    } else if (e.key === '2') {
      e.preventDefault();
      browseView.set('library');
      goto('/');
    } else if (e.key === '3') {
      e.preventDefault();
      goto('/settings');
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
  <main class="page" class:full={fullBleed}>
    {@render children()}
  </main>
{/if}
<OpStack />
<AppUpdateToast />
<ContextMenu />
<CloseToTrayPrompt />
<ShortcutsHelp />
<ResizeHandles />
<WingetBusy />
<ConfirmDialog />

<style>
  .page {
    max-width: 1180px;
    margin: 0 auto;
    padding: 26px 24px 90px;
  }
  .page.full {
    max-width: none;
    margin: 0;
    padding: 0;
    height: calc(100vh - 33px);
    display: flex;
    flex-direction: column;
  }
</style>

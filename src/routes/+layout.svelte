<script lang="ts">
  import '@fontsource/ibm-plex-sans/400.css';
  import '@fontsource/ibm-plex-sans/500.css';
  import '@fontsource/ibm-plex-sans/600.css';
  import '@fontsource/ibm-plex-mono/400.css';
  import '@fontsource/ibm-plex-mono/500.css';
  import '$lib/styles/app.css';

  import { onMount } from 'svelte';
  import { get } from 'svelte/store';
  import { goto, afterNavigate } from '$app/navigation';
  import Sidebar from '$lib/components/Sidebar.svelte';
  import QueueTray from '$lib/components/QueueTray.svelte';
  import OpStack from '$lib/components/OpStack.svelte';
  import ContextMenu from '$lib/components/ContextMenu.svelte';
  import CloseToTrayPrompt from '$lib/components/CloseToTrayPrompt.svelte';
  import ShortcutsHelp, { toggleShortcuts } from '$lib/components/ShortcutsHelp.svelte';
  import WingetBusy from '$lib/components/WingetBusy.svelte';
  import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';
  import Setup from '$lib/components/Setup.svelte';
  import { loadManagers } from '$lib/stores/managers';
  import { loadUpdates } from '$lib/stores/library';
  import { syncCatalog } from '$lib/stores/curated';
  import { settings } from '$lib/stores/settings';
  import { searchQuery } from '$lib/stores/search';
  import { initTray } from '$lib/stores/tray';
  import { initAppUpdater } from '$lib/stores/updater';
  import '$lib/stores/theme';

  let { children } = $props();

  const UPDATE_POLL_MS = 30 * 60 * 1000;

  // Ctrl+1..4 go to the sidebar's sections, Ctrl+, to Settings (as in most Windows apps).
  const SECTION_KEYS: Record<string, string> = {
    '1': '/',
    '2': '/installed',
    '3': '/updates',
    '4': '/activity',
    ',': '/settings'
  };

  function isTyping(e: KeyboardEvent) {
    const t = e.target as HTMLElement | null;
    return t?.tagName === 'INPUT' || t?.tagName === 'TEXTAREA' || t?.isContentEditable === true;
  }

  function focusSearch() {
    const input = document.getElementById('global-search') as HTMLInputElement | null;
    input?.focus();
    input?.select();
  }

  function onKey(e: KeyboardEvent) {
    if (!get(settings).setupComplete) return;
    const ctrl = (e.ctrlKey || e.metaKey) && !e.altKey && !e.shiftKey;
    if (e.key === '?' && !isTyping(e)) {
      e.preventDefault();
      toggleShortcuts();
    } else if ((ctrl && e.key.toLowerCase() === 'k') || (e.key === '/' && !isTyping(e))) {
      e.preventDefault();
      focusSearch();
    } else if (ctrl && SECTION_KEYS[e.key]) {
      e.preventDefault();
      goto(SECTION_KEYS[e.key]);
    }
  }

  // The sidebar search box mirrors the search page; leaving it clears the box.
  afterNavigate(({ to }) => {
    if (to?.url.pathname !== '/search') searchQuery.set('');
  });

  onMount(() => {
    document.documentElement.setAttribute('data-started', '');
    initTray();
    initAppUpdater();
    window.addEventListener('keydown', onKey);
    (async () => {
      await loadManagers();
      if (get(settings).refreshOnStartup) loadUpdates();
    })();
    syncCatalog();
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
  <div class="shell">
    <Sidebar />
    <div class="content">
      <main class="page">
        {@render children()}
      </main>
      <QueueTray />
    </div>
  </div>
{/if}
<OpStack />
<ContextMenu />
<CloseToTrayPrompt />
<ShortcutsHelp />
<WingetBusy />
<ConfirmDialog />

<style>
  .shell {
    display: flex;
    height: 100vh;
  }
  /* Pages sit on a raised panel beside the sidebar, its top-left corner
     rounded where it meets the title bar (as in WinUI apps). */
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
  .page {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
</style>

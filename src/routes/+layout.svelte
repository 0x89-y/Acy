<script lang="ts">
  import '@fontsource/ibm-plex-sans/400.css';
  import '@fontsource/ibm-plex-sans/500.css';
  import '@fontsource/ibm-plex-sans/600.css';
  import '@fontsource/ibm-plex-mono/400.css';
  import '@fontsource/ibm-plex-mono/500.css';
  import '$lib/styles/app.css';

  import { onMount } from 'svelte';
  import Nav from '$lib/components/Nav.svelte';
  import OpStack from '$lib/components/OpStack.svelte';
  import ResizeHandles from '$lib/components/ResizeHandles.svelte';
  import { loadManagers } from '$lib/stores/managers';
  import '$lib/stores/theme';

  let { children } = $props();

  onMount(() => {
    loadManagers();
  });
</script>

<Nav />
<main class="page">
  {@render children()}
</main>
<OpStack />
<ResizeHandles />

<style>
  .page {
    max-width: 1180px;
    margin: 0 auto;
    padding: 26px 24px 90px;
  }
</style>

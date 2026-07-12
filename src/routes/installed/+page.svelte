<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { browseView } from '$lib/stores/discover';

  // Installed + Updates now live inside the Discover panel's "Library" mode.
  // Replace history so Back doesn't bounce through this redirect.
  onMount(() => {
    browseView.set('library');
    goto('/', { replaceState: true });
  });
</script>

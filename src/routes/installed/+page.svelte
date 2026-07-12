<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { browseView } from '$lib/stores/discover';

  onMount(() => {
    browseView.set('library');
    goto('/', { replaceState: true });
  });
</script>

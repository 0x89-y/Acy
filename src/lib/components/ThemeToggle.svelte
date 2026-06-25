<script lang="ts">
  import { get } from 'svelte/store';
  import { appliedTheme } from '$lib/stores/theme';
  import { setThemeMode } from '$lib/stores/settings';
  import { Sun, Moon } from '@lucide/svelte';

  function toggle() {
    setThemeMode(get(appliedTheme) === 'dark' ? 'light' : 'dark');
  }
</script>

<button
  class="btn btn-ghost icon-btn"
  onclick={toggle}
  title="Toggle light / dark"
  aria-label="Toggle light or dark theme"
>
  {#if $appliedTheme === 'dark'}
    <Sun size={18} />
  {:else}
    <Moon size={18} />
  {/if}
</button>

<style>
  .icon-btn {
    padding: 8px;
    line-height: 0;
  }
</style>

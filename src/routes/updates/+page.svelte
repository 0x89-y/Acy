<script lang="ts">
  import UpdatesSection from '$lib/components/UpdatesSection.svelte';
  import { loadUpdates } from '$lib/stores/library';

  $effect(() => {
    loadUpdates();
  });
</script>

<div class="view">
  <header class="view-head">
    <div>
      <h1>Updates</h1>
      <p class="view-sub">Newer versions of apps you have installed.</p>
    </div>
  </header>
  <UpdatesSection />
</div>

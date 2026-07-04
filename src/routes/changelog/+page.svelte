<script lang="ts">
  import { ArrowLeft, ChevronLeft, ChevronRight } from '@lucide/svelte';
  import { CHANGELOG } from '$lib/changelog';

  const PAGE_SIZE = 4;
  let page = $state(0);
  let pageCount = $derived(Math.max(1, Math.ceil(CHANGELOG.length / PAGE_SIZE)));
  let releases = $derived(CHANGELOG.slice(page * PAGE_SIZE, (page + 1) * PAGE_SIZE));

  function go(next: number) {
    page = Math.max(0, Math.min(pageCount - 1, next));
    window.scrollTo({ top: 0, behavior: 'smooth' });
  }
</script>

<a class="back" href="/settings"><ArrowLeft size={16} /> Settings</a>

<header>
  <div>
    <h1>Changelog</h1>
  </div>
  <span class="page-count mono">Page {page + 1} of {pageCount}</span>
</header>

<div class="releases">
  {#each releases as release (release.version)}
    <article class="release card">
      <div class="release-head">
        <h2 class="mono">v{release.version}</h2>
        <time class="mono muted" datetime={release.date}>{release.date}</time>
      </div>
      <ul>
        {#each release.changes as change (change)}
          <li>{change}</li>
        {/each}
      </ul>
    </article>
  {/each}
</div>

{#if pageCount > 1}
  <nav class="pagination" aria-label="Changelog pages">
    <div class="seg-actions">
      <button class="seg-act" onclick={() => go(page - 1)} disabled={page === 0}>
        <ChevronLeft size={15} /> Newer
      </button>
      <button class="seg-act" onclick={() => go(page + 1)} disabled={page === pageCount - 1}>
        Older <ChevronRight size={15} />
      </button>
    </div>
  </nav>
{/if}

<style>
  .back {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    margin-bottom: 18px;
    color: var(--text-muted);
    font-size: 0.9rem;
    text-decoration: none;
  }
  .back:hover {
    color: var(--text);
  }
  header {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 16px;
    margin-bottom: 20px;
  }
  .page-count {
    color: var(--text-muted);
    font-size: 0.76rem;
  }
  .releases {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .release {
    padding: 18px 20px;
  }
  .release-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 10px;
  }
  .release h2 {
    font-size: 1rem;
  }
  .release time {
    font-size: 0.74rem;
  }
  .release ul {
    margin: 0;
    padding-left: 20px;
    color: var(--text-muted);
    font-size: 0.88rem;
  }
  .release li + li {
    margin-top: 5px;
  }
  .pagination {
    display: flex;
    justify-content: center;
    margin-top: 18px;
  }
</style>

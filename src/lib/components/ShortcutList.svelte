<script module lang="ts">
  const shortcuts: { keys: string[]; desc: string }[] = [
    { keys: ['Ctrl', 'K'], desc: 'Focus search (or /)' },
    { keys: ['Enter'], desc: 'Search package managers (in the search box)' },
    { keys: ['Ctrl', '1'], desc: 'Go to Discover' },
    { keys: ['Ctrl', '2'], desc: 'Go to Library (installed & updates)' },
    { keys: ['Ctrl', '3'], desc: 'Go to Settings' },
    { keys: ['↑', '↓', '←', '→'], desc: 'Move between search results' },
    { keys: ['Enter'], desc: 'Open or install the focused result' },
    { keys: ['Ctrl', 'Click'], desc: 'Select an app' },
    { keys: ['Shift', 'Click'], desc: 'Select a range of apps' },
    { keys: ['Esc'], desc: 'Clear search or selection, go back, or close a dialog' },
    { keys: ['Right-click'], desc: 'App actions menu (cards and installed rows)' },
    { keys: ['?'], desc: 'Show keyboard shortcuts' }
  ];
</script>

<ul class="list">
  {#each shortcuts as s (s.desc)}
    <li>
      <span class="keys">
        {#each s.keys as k (k)}<kbd>{k}</kbd>{/each}
      </span>
      <span class="desc">{s.desc}</span>
    </li>
  {/each}
</ul>

<style>
  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .list li {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 7px 0;
    border-bottom: 1px solid var(--border);
  }
  .list li:last-child {
    border-bottom: none;
  }
  .keys {
    flex: 0 0 130px;
    display: flex;
    gap: 4px;
    flex-wrap: wrap;
  }
  kbd {
    font-family: var(--font-mono);
    font-size: 0.72rem;
    color: var(--text);
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 2px 7px;
    line-height: 1.4;
  }
  .desc {
    flex: 1;
    font-size: 0.88rem;
    color: var(--text-muted);
  }
</style>

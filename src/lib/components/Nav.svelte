<script lang="ts">
  import { page } from '$app/stores';
  import { Settings } from '@lucide/svelte';
  import ThemeToggle from './ThemeToggle.svelte';
  import WindowControls from './WindowControls.svelte';
  import { updatesCount } from '$lib/stores/managers';

  const links = [
    { href: '/', label: 'Discover' },
    { href: '/installed', label: 'Installed' }
  ];

  let path = $derived($page.url.pathname);
  function isActive(href: string) {
    return href === '/' ? path === '/' : path.startsWith(href);
  }
</script>

<header class="nav" data-tauri-drag-region>
  <div class="brand" data-tauri-drag-region>
    <span class="logo mono">acy</span>
    <span class="tag muted">app store</span>
  </div>
  <nav class="links">
    {#each links as l (l.href)}
      <a href={l.href} class="link" class:active={isActive(l.href)}>
        {l.label}
        {#if l.href === '/installed' && $updatesCount > 0}
          <span class="count">{$updatesCount}</span>
        {/if}
      </a>
    {/each}
  </nav>
  <div class="spacer" data-tauri-drag-region></div>
  <div class="right">
    <a
      href="/settings"
      class="icon-link"
      class:active={path.startsWith('/settings')}
      title="Settings"
      aria-label="Settings"
    >
      <Settings size={18} />
    </a>
    <ThemeToggle />
    <div class="win-divider"></div>
    <WindowControls />
  </div>
</header>

<style>
  .nav {
    position: sticky;
    top: 0;
    z-index: 20;
    display: flex;
    align-items: center;
    gap: 22px;
    padding: 12px 8px 12px 24px;
    background: color-mix(in srgb, var(--bg) 82%, transparent);
    backdrop-filter: blur(12px);
    border-bottom: 1px solid var(--border);
  }
  .brand {
    display: flex;
    align-items: baseline;
    gap: 8px;
  }
  .logo {
    font-size: 1.15rem;
    font-weight: 600;
    letter-spacing: 0.02em;
  }
  .logo::before {
    content: '0x';
    color: var(--accent);
  }
  .tag {
    font-size: 0.72rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }
  .links {
    display: flex;
    gap: 4px;
  }
  .link {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    border-radius: var(--radius-sm);
    color: var(--text-muted);
    text-decoration: none;
    font-weight: 500;
    font-size: 0.92rem;
  }
  .link:hover {
    background: var(--surface-hover);
    color: var(--text);
  }
  .link.active {
    color: var(--text);
    background: var(--surface);
    border: 1px solid var(--border);
  }
  .count {
    font-family: var(--font-mono);
    font-size: 0.7rem;
    background: var(--accent);
    color: var(--accent-contrast);
    border-radius: var(--radius-pill);
    padding: 0 6px;
    line-height: 1.5;
  }
  .spacer {
    flex: 1;
  }
  .right {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .win-divider {
    width: 1px;
    height: 20px;
    background: var(--border);
    margin: 0 6px;
  }
  .icon-link {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 8px;
    border-radius: var(--radius-sm);
    color: var(--text-muted);
    line-height: 0;
  }
  .icon-link:hover {
    background: var(--surface-hover);
    color: var(--text);
  }
  .icon-link.active {
    color: var(--text);
    background: var(--surface);
  }
</style>

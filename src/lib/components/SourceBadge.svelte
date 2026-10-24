<script lang="ts">
  import type { Source } from '$lib/types';
  import { launcherOf, LAUNCHER_LABELS } from '$lib/installedGroups';
  let { source, id = null }: { source: Source; id?: string | null } = $props();

  // Apps installed by a game launcher show the launcher, not the winget record
  // they happen to be listed through. Launchers are product names, so they're
  // capitalized; the managers stay lowercase like the commands they are.
  let launcher = $derived(id ? launcherOf(source, id) : null);
  let label = $derived(launcher ? LAUNCHER_LABELS[launcher] : source);
  let title = $derived(
    launcher ? `Installed by ${label} (listed through winget)` : `Source: ${source}`
  );
</script>

<span class="badge {launcher ?? source}" {title}>{label}</span>

<style>
  .badge {
    font-family: var(--font-mono);
    font-size: 0.68rem;
    padding: 1px 7px;
    border-radius: var(--radius-sm);
    border: 1px solid currentColor;
    line-height: 1.5;
    white-space: nowrap;
  }
  .winget {
    color: var(--winget);
  }
  .scoop {
    color: var(--scoop);
  }
  .choco {
    color: var(--choco);
  }
  .msstore {
    color: var(--msstore);
  }
  .steam {
    color: var(--steam);
  }
  .epic {
    color: var(--epic);
  }
  .riot {
    color: var(--riot);
  }
  .rockstar {
    color: var(--rockstar);
  }
</style>

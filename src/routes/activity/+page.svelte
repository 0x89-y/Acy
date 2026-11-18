<script lang="ts">
  import { Download, RefreshCw, Trash2, WandSparkles, CircleAlert, History } from '@lucide/svelte';
  import { activity, clearActivity, type ActivityAction, type ActivityEntry } from '$lib/stores/activity';
  import { confirmAction } from '$lib/stores/confirm';
  import type { Source } from '$lib/types';

  const VERB: Record<ActivityAction, string> = {
    install: 'Installed',
    update: 'Updated',
    uninstall: 'Removed',
    'update-all': 'Updated all',
    setup: 'Set up'
  };
  const FAILED: Record<ActivityAction, string> = {
    install: 'Install failed',
    update: 'Update failed',
    uninstall: 'Removal failed',
    'update-all': 'Updating all failed',
    setup: 'Setup failed'
  };
  const ICON = {
    install: Download,
    update: RefreshCw,
    uninstall: Trash2,
    'update-all': RefreshCw,
    setup: WandSparkles
  } satisfies Record<ActivityAction, typeof Download>;
  const SOURCE_NAMES: Record<Source, string> = {
    winget: 'winget',
    scoop: 'Scoop',
    choco: 'Chocolatey',
    msstore: 'Microsoft Store'
  };

  function dayKey(at: number) {
    const d = new Date(at);
    return `${d.getFullYear()}-${d.getMonth()}-${d.getDate()}`;
  }
  function dayLabel(at: number): string {
    const today = new Date();
    const yesterday = new Date();
    yesterday.setDate(today.getDate() - 1);
    if (dayKey(at) === dayKey(today.getTime())) return 'Today';
    if (dayKey(at) === dayKey(yesterday.getTime())) return 'Yesterday';
    return new Date(at).toLocaleDateString(undefined, {
      weekday: 'long',
      month: 'long',
      day: 'numeric',
      year: new Date(at).getFullYear() === today.getFullYear() ? undefined : 'numeric'
    });
  }
  const time = (at: number) =>
    new Date(at).toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' });

  // Entries are newest first; group consecutive ones by calendar day.
  let days = $derived.by(() => {
    const out: { key: string; label: string; entries: ActivityEntry[] }[] = [];
    for (const e of $activity) {
      const k = dayKey(e.at);
      if (out.at(-1)?.key !== k) out.push({ key: k, label: dayLabel(e.at), entries: [] });
      out.at(-1)!.entries.push(e);
    }
    return out;
  });
  let failedCount = $derived($activity.filter((e) => !e.ok).length);

  async function clearAll() {
    const ok = await confirmAction({
      title: 'Clear activity?',
      message: 'The history of installs, updates and removals is deleted. Your apps are not affected.',
      confirmLabel: 'Clear',
      danger: true
    });
    if (ok) clearActivity();
  }
</script>

<div class="view">
  <header class="view-head">
    <div>
      <h1>Activity</h1>
      <p class="view-sub">
        {#if $activity.length === 0}
          Installs, updates and removals made with Acy.
        {:else}
          {$activity.length} {$activity.length === 1 ? 'entry' : 'entries'}{failedCount > 0
            ? ` · ${failedCount} failed`
            : ''} · running operations show in the bar below
        {/if}
      </p>
    </div>
    {#if $activity.length > 0}
      <button class="btn btn-ghost" onclick={clearAll}><Trash2 size={15} /> Clear activity</button>
    {/if}
  </header>

  <div class="view-body">
    <div class="page">
      {#if $activity.length === 0}
        <div class="empty">
          <History size={28} aria-hidden="true" />
          <h2>Nothing here yet</h2>
          <p class="muted">Apps you install, update or remove with Acy will be listed here.</p>
        </div>
      {:else}
        {#each days as day (day.key)}
          <section class="group" aria-labelledby={`day-${day.key}`}>
            <h2 id={`day-${day.key}`}>{day.label}</h2>
            <ul class="entries">
              {#each day.entries as e (e.id)}
                {@const Icon = e.ok ? ICON[e.action] : CircleAlert}
                <li class="entry" class:failed={!e.ok}>
                  <span class="ic" aria-hidden="true"><Icon size={18} /></span>
                  <span class="text">
                    <span class="name">{e.name}</span>
                    <span class="meta">
                      {e.ok ? VERB[e.action] : FAILED[e.action]}{e.source ? ` with ${SOURCE_NAMES[e.source]}` : ''}
                    </span>
                  </span>
                  <time class="time mono" datetime={new Date(e.at).toISOString()}>{time(e.at)}</time>
                </li>
              {/each}
            </ul>
          </section>
        {/each}
      {/if}
    </div>
  </div>
</div>

<style>
  .page {
    max-width: 880px;
    padding: 0 24px 32px;
    display: flex;
    flex-direction: column;
    gap: 22px;
  }
  .group h2 {
    margin: 0 0 6px;
    font-size: var(--fs-base);
    font-weight: 600;
  }
  .entries {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .entry {
    display: flex;
    align-items: center;
    gap: 14px;
    min-height: 56px;
    padding: 8px 16px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface-2);
  }
  .ic {
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    border-radius: var(--radius-sm);
    background: color-mix(in srgb, var(--accent) 14%, transparent);
    color: var(--accent);
  }
  .failed .ic {
    background: color-mix(in srgb, var(--danger) 14%, transparent);
    color: var(--danger);
  }
  .text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .name {
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .meta {
    font-size: var(--fs-sm);
    color: var(--text-muted);
  }
  .failed .meta {
    color: var(--danger);
  }
  .time {
    flex-shrink: 0;
    font-size: var(--fs-sm);
    color: var(--text-muted);
  }
  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    margin-top: 48px;
    text-align: center;
    color: var(--text-muted);
  }
  .empty h2 {
    margin: 6px 0 0;
    font-size: var(--fs-lg);
    color: var(--text);
  }
  .empty p {
    margin: 0;
    font-size: var(--fs-md);
  }
</style>

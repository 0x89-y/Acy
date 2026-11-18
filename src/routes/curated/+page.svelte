<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { beforeNavigate, goto } from '$app/navigation';
  import {
    Search,
    X,
    Plus,
    Trash2,
    Ellipsis,
    FolderPlus,
    ChevronLeft,
    Type,
    AlignLeft,
    Tags,
    Image,
    House,
    ScrollText,
    Heart,
    Package,
    Folder
  } from '@lucide/svelte';
  import AppIcon from '$lib/components/AppIcon.svelte';
  import AppIconControls from '$lib/components/AppIconControls.svelte';
  import SettingCard from '$lib/components/SettingCard.svelte';
  import * as api from '$lib/api';
  import { settings } from '$lib/stores/settings';
  import { reloadCurated } from '$lib/stores/curated';
  import { confirmAction } from '$lib/stores/confirm';
  import { notice } from '$lib/stores/ops';
  import { openContextMenuAt, type CtxItem } from '$lib/stores/contextMenu';
  import type { CuratedFile, CuratedApp, CuratedCategory, Source } from '$lib/types';

  const sources: Source[] = ['winget', 'scoop', 'choco', 'msstore'];
  const SOURCE_NAMES: Record<Source, string> = {
    winget: 'winget',
    scoop: 'Scoop',
    choco: 'Chocolatey',
    msstore: 'Microsoft Store'
  };

  let file = $state<CuratedFile | null>(null);
  let loading = $state(true);
  let loadError = $state<string | null>(null);
  let saving = $state(false);
  let saveError = $state<string | null>(null);

  // What was last loaded/saved; anything different is unsaved.
  let baseline = $state('');
  let dirty = $derived(file !== null && JSON.stringify(file) !== baseline);

  let selectedCat = $state<number | 'all'>('all');
  let activeCat = $derived(
    typeof selectedCat === 'number' ? (file?.categories[selectedCat] ?? null) : null
  );
  $effect(() => {
    const n = file?.categories.length ?? 0;
    if (typeof selectedCat === 'number' && selectedCat > n - 1) selectedCat = 'all';
  });

  // The app being edited. Editing is a view inside this page (not a route), so
  // unsaved changes in the list and the editor are one set of changes.
  let editing = $state<{ ci: number; ai: number } | null>(null);
  let editCat = $derived(editing ? (file?.categories[editing.ci] ?? null) : null);
  let editApp = $derived(editing && editCat ? (editCat.apps[editing.ai] ?? null) : null);
  let heading = $state<HTMLHeadingElement | null>(null);
  let scroller = $state<HTMLDivElement | null>(null);
  let listScroll = 0;

  async function openEditor(ci: number, ai: number) {
    listScroll = scroller?.scrollTop ?? 0;
    editing = { ci, ai };
    await tick();
    if (scroller) scroller.scrollTop = 0;
    heading?.focus();
  }
  async function closeEditor() {
    editing = null;
    await tick();
    if (scroller) scroller.scrollTop = listScroll;
  }

  let filter = $state('');
  let filtering = $derived(filter.trim().length > 0);
  function matchesApp(app: CuratedApp): boolean {
    const q = filter.trim().toLowerCase();
    if (!q) return true;
    const hay = [app.name ?? '', app.id, app.source, ...(app.tags ?? []), ...app.alternates.map((a) => a.id)]
      .join(' ')
      .toLowerCase();
    return hay.includes(q);
  }
  function catMatches(cat: CuratedCategory): boolean {
    const q = filter.trim().toLowerCase();
    if (!q) return true;
    return `${cat.title} ${cat.id}`.toLowerCase().includes(q);
  }

  // With curated apps turned off, only the user's own apps are listed.
  let hideBuiltins = $derived(!$settings.showCuratedApps);
  const appVisible = (cat: CuratedCategory, app: CuratedApp) =>
    (!filtering || catMatches(cat) || matchesApp(app)) && (!hideBuiltins || app.custom);
  const shownCount = (cat: CuratedCategory) =>
    hideBuiltins ? cat.apps.filter((a) => a.custom).length : cat.apps.length;

  onMount(async () => {
    try {
      file = await api.getCurated();
      baseline = JSON.stringify(file);
    } catch (e) {
      loadError = `Could not load the catalog: ${e}`;
    } finally {
      loading = false;
    }
  });

  // Leaving with unsaved edits asks first (sidebar, shortcuts).
  let leaving = false;
  beforeNavigate((nav) => {
    if (!dirty || leaving || !nav.to) return;
    nav.cancel();
    const target = nav.to.url;
    confirmAction({
      title: 'Discard unsaved changes?',
      message: 'Your edits to the app catalog haven’t been saved.',
      confirmLabel: 'Discard',
      danger: true
    }).then((ok) => {
      if (!ok) return;
      leaving = true;
      goto(target.pathname + target.search);
    });
  });

  function move<T>(arr: T[], i: number, delta: number): boolean {
    const j = i + delta;
    if (j < 0 || j >= arr.length) return false;
    [arr[i], arr[j]] = [arr[j], arr[i]];
    return true;
  }

  function addCategory() {
    if (!file) return;
    file.categories.push({ id: '', title: '', apps: [] });
    selectedCat = file.categories.length - 1;
    editing = null;
  }
  async function removeCategory(ci: number) {
    const cat = file?.categories[ci];
    if (!cat) return;
    if (cat.apps.length > 0) {
      const ok = await confirmAction({
        title: `Remove “${cat.title || cat.id || 'this category'}”?`,
        message: `Its ${cat.apps.length} ${cat.apps.length === 1 ? 'app is' : 'apps are'} removed from your list too. Nothing is saved until you press Save.`,
        confirmLabel: 'Remove',
        danger: true
      });
      if (!ok) return;
    }
    file?.categories.splice(ci, 1);
  }
  function moveCategory(ci: number, delta: number) {
    if (!file) return;
    move(file.categories, ci, delta);
    if (selectedCat === ci) selectedCat = Math.max(0, Math.min(file.categories.length - 1, ci + delta));
  }
  function addApp(cat: CuratedCategory, ci: number) {
    cat.apps.push({
      id: '',
      source: 'winget',
      name: null,
      description: null,
      homepage: null,
      icon: null,
      alternates: [],
      tags: [],
      donate: null,
      releaseNotes: null,
      custom: true
    });
    openEditor(ci, cat.apps.length - 1);
  }
  function addAlternate(app: CuratedApp) {
    const used = new Set<Source>([app.source, ...app.alternates.map((v) => v.source)]);
    const next = sources.find((s) => !used.has(s));
    if (next) app.alternates.push({ source: next, id: '' });
  }
  function removeAlternate(app: CuratedApp, i: number) {
    app.alternates.splice(i, 1);
  }
  function removePrimary(app: CuratedApp) {
    const next = app.alternates.shift();
    if (next) {
      app.source = next.source;
      app.id = next.id;
    }
  }
  // The default source swaps places with an alternate.
  function makeDefault(app: CuratedApp, k: number) {
    const alt = app.alternates[k];
    app.alternates[k] = { source: app.source, id: app.id };
    app.source = alt.source;
    app.id = alt.id;
  }
  function addTag(app: CuratedApp, raw: string) {
    const have = new Set(app.tags ?? []);
    const fresh = raw
      .split(',')
      .map((t) => t.trim())
      .filter((t) => t && !have.has(t) && have.add(t));
    if (fresh.length) app.tags = [...(app.tags ?? []), ...fresh];
  }
  function removeTag(app: CuratedApp, k: number) {
    app.tags = (app.tags ?? []).filter((_, i) => i !== k);
  }
  function onTagKey(e: KeyboardEvent & { currentTarget: HTMLInputElement }, app: CuratedApp) {
    const input = e.currentTarget;
    if (e.key === 'Enter' || e.key === ',') {
      e.preventDefault();
      addTag(app, input.value);
      input.value = '';
    } else if (e.key === 'Backspace' && !input.value && (app.tags ?? []).length) {
      removeTag(app, (app.tags ?? []).length - 1);
    }
  }

  // The editor's Category dropdown: the app goes to the end of the new category
  // and the editor follows it.
  function changeCategory(to: number) {
    if (!file || !editing || to === editing.ci) return;
    const [app] = file.categories[editing.ci].apps.splice(editing.ai, 1);
    file.categories[to].apps.push(app);
    editing = { ci: to, ai: file.categories[to].apps.length - 1 };
  }

  // Menu actions keep the editor pointing at the same app as it moves.
  function appMenuItems(ci: number, ai: number, withMoveTo = true): CtxItem[] {
    const cat = file!.categories[ci];
    const follow = (to: { ci: number; ai: number }) => {
      if (editing && editing.ci === ci && editing.ai === ai) editing = to;
    };
    const items: CtxItem[] = [
      { label: 'Move up', disabled: ai === 0, onSelect: () => move(cat.apps, ai, -1) && follow({ ci, ai: ai - 1 }) },
      {
        label: 'Move down',
        disabled: ai === cat.apps.length - 1,
        onSelect: () => move(cat.apps, ai, 1) && follow({ ci, ai: ai + 1 })
      }
    ];
    for (const [to, c] of file!.categories.entries()) {
      if (to === ci || !withMoveTo) continue;
      items.push({
        label: `Move to ${c.title || c.id || 'Untitled'}`,
        onSelect: () => {
          const [app] = cat.apps.splice(ai, 1);
          c.apps.push(app);
          follow({ ci: to, ai: c.apps.length - 1 });
        }
      });
    }
    items.push({
      label: 'Remove from list',
      danger: true,
      onSelect: () => {
        const wasOpen = editing?.ci === ci && editing?.ai === ai;
        cat.apps.splice(ai, 1);
        if (wasOpen) closeEditor();
      }
    });
    return items;
  }

  function categoryMenu(e: MouseEvent & { currentTarget: HTMLElement }, ci: number) {
    const last = (file?.categories.length ?? 1) - 1;
    openContextMenuAt(e.currentTarget, [
      { label: 'Move up', disabled: ci === 0, onSelect: () => moveCategory(ci, -1) },
      { label: 'Move down', disabled: ci === last, onSelect: () => moveCategory(ci, 1) },
      { label: 'Remove category…', danger: true, onSelect: () => removeCategory(ci) }
    ]);
  }

  function clean(file: CuratedFile): CuratedFile {
    const blank = (v: string | null) => (v && v.trim() ? v.trim() : null);
    return {
      ...file,
      categories: file.categories.map((c) => ({
        id: c.id.trim(),
        title: c.title.trim(),
        apps: c.apps.map((a) => {
          const seen = new Set<Source>();
          const uniq = [{ source: a.source, id: a.id }, ...a.alternates]
            .map((v) => ({ source: v.source, id: v.id.trim() }))
            .filter((v) => v.id && !seen.has(v.source) && seen.add(v.source));
          const primary = uniq[0] ?? { source: a.source, id: '' };
          return {
            id: primary.id,
            source: primary.source,
            name: blank(a.name),
            description: blank(a.description),
            homepage: blank(a.homepage),
            icon: blank(a.icon),
            alternates: uniq.slice(1),
            tags: (a.tags ?? []).map((t) => t.trim()).filter(Boolean),
            donate: blank(a.donate),
            releaseNotes: blank(a.releaseNotes),
            custom: a.custom
          };
        })
      }))
    };
  }

  function validate(f: CuratedFile): string | null {
    for (const c of f.categories) {
      if (!c.id || !c.title) return 'Every category needs a title and an id.';
      for (const a of c.apps) {
        if (!a.id) return `An app in “${c.title}” is missing its package id.`;
      }
    }
    return null;
  }

  async function save() {
    if (!file) return;
    const cleaned = clean(file);
    const err = validate(cleaned);
    if (err) {
      saveError = err;
      return;
    }
    saving = true;
    saveError = null;
    try {
      await api.saveCurated(cleaned);
      file = cleaned;
      baseline = JSON.stringify(cleaned);
      await reloadCurated();
      notice('Saved. Your changes show on Discover.', 'ok');
    } catch (e) {
      saveError = `Save failed: ${e}`;
    } finally {
      saving = false;
    }
  }

  function discard() {
    if (!baseline) return;
    file = JSON.parse(baseline);
    saveError = null;
    editing = null;
  }

  let appCount = $derived(file?.categories.reduce((n, c) => n + shownCount(c), 0) ?? 0);
  let listed = $derived.by(() => {
    const out: { cat: CuratedCategory; ci: number; app: CuratedApp; ai: number }[] = [];
    file?.categories.forEach((cat, ci) => {
      if (typeof selectedCat === 'number' && ci !== selectedCat) return;
      cat.apps.forEach((app, ai) => out.push({ cat, ci, app, ai }));
    });
    return out.filter((e) => appVisible(e.cat, e.app));
  });

  const SOURCE_LETTERS: Record<Source, string> = { winget: 'W', scoop: 'S', choco: 'C', msstore: 'M' };
  const appSources = (app: CuratedApp): Source[] => [app.source, ...app.alternates.map((a) => a.source)];
  const sourceLabel = (app: CuratedApp) =>
    'Available from ' +
    appSources(app)
      .map((s, i) => SOURCE_NAMES[s] + (i === 0 && app.alternates.length ? ' (default)' : ''))
      .join(', ');
</script>

<!-- Fixed slots (W S C M) so the letters line up in columns down the list; a
     missing manager leaves its slot empty. The default is the filled dot.
     Letters keep them readable without telling the colours apart. -->
{#snippet sourceBadges(app: CuratedApp)}
  {@const have = new Set(appSources(app))}
  <span class="src-dots" role="img" aria-label={sourceLabel(app)} title={sourceLabel(app)}>
    {#each sources as s (s)}
      {#if have.has(s) && (app.id || app.alternates.length)}
        <span class="sd {s}" class:default={s === app.source} aria-hidden="true">{SOURCE_LETTERS[s]}</span>
      {:else}
        <span class="sd-empty" aria-hidden="true"></span>
      {/if}
    {/each}
  </span>
{/snippet}

<div class="view">
  {#if editing && editApp && editCat}
    {@const app = editApp}
    {@const ci = editing.ci}
    {@const ai = editing.ai}
    <header class="view-head">
      <div>
        <button class="crumb" onclick={closeEditor}><ChevronLeft size={14} /> App catalog</button>
        <div class="title-row">
          <AppIcon name={app.name || app.id || '?'} size={40} source={app.source} id={app.id} homepage={app.icon || app.homepage} />
          <div>
            <h1 tabindex="-1" bind:this={heading}>{app.name || app.id || 'New app'}</h1>
            <p class="view-sub">{editCat.title || editCat.id || 'Untitled category'}{app.custom ? ' · Added by you' : ''}</p>
          </div>
        </div>
      </div>
      <button
        class="btn"
        aria-haspopup="menu"
        onclick={(e) => {
          e.stopPropagation();
          openContextMenuAt(e.currentTarget, appMenuItems(ci, ai, false));
        }}
      >
        <Ellipsis size={16} /> More
      </button>
    </header>

    <div class="view-body" bind:this={scroller}>
      <div class="page">
        <section class="group" aria-labelledby="ed-listing">
          <h2 id="ed-listing">Listing</h2>
          <SettingCard icon={Type} title="Name" description="Shown on Discover and on the app's page.">
            {#snippet control()}
              <input class="text-input field" placeholder={app.id || 'App name'} aria-label="Name" bind:value={app.name} />
            {/snippet}
          </SettingCard>
          <SettingCard icon={AlignLeft} title="Description" description="One short sentence under the name.">
            {#snippet control()}
              <input class="text-input field" placeholder="What the app does" aria-label="Description" bind:value={app.description} />
            {/snippet}
          </SettingCard>
          <SettingCard icon={Folder} title="Category" titleId="ed-cat" description="Which Discover section the app is listed in.">
            {#snippet control()}
              <select class="text-input field" aria-labelledby="ed-cat" value={ci} onchange={(e) => changeCategory(Number(e.currentTarget.value))}>
                {#each file?.categories ?? [] as c, k (k)}
                  <option value={k}>{c.title || c.id || 'Untitled'}</option>
                {/each}
              </select>
            {/snippet}
          </SettingCard>
          <SettingCard icon={Tags} title="Tags" description="Used by the tag filter. Press Enter or comma to add one.">
            {#snippet control()}
              <div class="chips-input field" role="group" aria-label="Tags">
                {#each app.tags ?? [] as t, k (k)}
                  <span class="chip">
                    {t}
                    <button class="chip-x" onclick={() => removeTag(app, k)} aria-label={`Remove tag ${t}`}><X size={12} /></button>
                  </span>
                {/each}
                <input
                  placeholder={(app.tags ?? []).length ? 'Add' : 'open source, free…'}
                  aria-label="Add a tag"
                  onkeydown={(e) => onTagKey(e, app)}
                  onblur={(e) => {
                    addTag(app, e.currentTarget.value);
                    e.currentTarget.value = '';
                  }}
                />
              </div>
            {/snippet}
          </SettingCard>
          <SettingCard icon={Image} title="Icon" description="An image or page URL. Leave empty to use the homepage's icon.">
            {#snippet control()}
              <input class="text-input field" placeholder="https://…" aria-label="Icon URL" bind:value={app.icon} />
            {/snippet}
            <AppIconControls source={app.source} id={app.id} name={app.name ?? ''} from={app.icon || app.homepage || null} size={40} />
          </SettingCard>
        </section>

        <section class="group" aria-labelledby="ed-sources">
          <h2 id="ed-sources">Install from</h2>
          <SettingCard
            icon={Package}
            title="Package managers"
            description="Install uses the default. The others are offered on the app's page."
          >
            <ul class="src-list">
              <li class="src-row">
                <select class="text-input" bind:value={app.source} aria-label="Package manager">
                  {#each sources as s (s)}<option value={s}>{SOURCE_NAMES[s]}</option>{/each}
                </select>
                <input class="text-input mono" placeholder="Package id" aria-label="Package id" bind:value={app.id} />
                <span class="default-tag">Default</span>
                <button class="icon-button" title="Remove" aria-label="Remove this source" disabled={app.alternates.length === 0} onclick={() => removePrimary(app)}>
                  <Trash2 size={15} />
                </button>
              </li>
              {#each app.alternates as alt, k (k)}
                <li class="src-row">
                  <select class="text-input" bind:value={alt.source} aria-label="Package manager">
                    {#each sources as s (s)}<option value={s}>{SOURCE_NAMES[s]}</option>{/each}
                  </select>
                  <input class="text-input mono" placeholder="Package id" aria-label="Package id" bind:value={alt.id} />
                  <button class="btn btn-ghost small quiet" onclick={() => makeDefault(app, k)}>Make default</button>
                  <button class="icon-button" title="Remove" aria-label="Remove this source" onclick={() => removeAlternate(app, k)}>
                    <Trash2 size={15} />
                  </button>
                </li>
              {/each}
            </ul>
            <button class="btn btn-ghost small add-src" onclick={() => addAlternate(app)} disabled={1 + app.alternates.length >= sources.length}>
              <Plus size={14} /> Add another source
            </button>
          </SettingCard>
        </section>

        <section class="group" aria-labelledby="ed-links">
          <h2 id="ed-links">Links</h2>
          <SettingCard icon={House} title="Homepage" description="Also where the icon comes from when no icon URL is set.">
            {#snippet control()}
              <input class="text-input field" placeholder="https://…" aria-label="Homepage" bind:value={app.homepage} />
            {/snippet}
          </SettingCard>
          <SettingCard icon={ScrollText} title="Release notes" description="Linked from the app's page and from Updates.">
            {#snippet control()}
              <input class="text-input field" placeholder="https://…" aria-label="Release notes" bind:value={app.releaseNotes} />
            {/snippet}
          </SettingCard>
          <SettingCard icon={Heart} title="Donate" description="Shown as a Donate button on the app's page.">
            {#snippet control()}
              <input class="text-input field" placeholder="https://…" aria-label="Donate" bind:value={app.donate} />
            {/snippet}
          </SettingCard>
        </section>
      </div>
    </div>
  {:else}
    <header class="view-head">
      <div>
        <a class="crumb" href="/settings"><ChevronLeft size={14} /> Settings</a>
        <h1>App catalog</h1>
        <p class="view-sub">
          {#if file}
            {appCount} apps in {file.categories.length} categories. Changes show on Discover once you save.
          {:else}
            The apps and categories on Discover.
          {/if}
        </p>
      </div>
    </header>

    {#if file}
      <div class="toolbar">
        <div class="filter">
          <Search size={15} aria-hidden="true" />
          <input placeholder="Filter by name, id or tag" aria-label="Filter apps" bind:value={filter} />
          {#if filter}
            <button class="icon-button tiny" onclick={() => (filter = '')} aria-label="Clear filter"><X size={14} /></button>
          {/if}
        </div>
        <label class="cat-pick">
          <span class="muted">Category</span>
          <select class="text-input" bind:value={selectedCat}>
            <option value="all">All apps ({appCount})</option>
            {#each file.categories as cat, ci (ci)}
              <option value={ci}>{cat.title || cat.id || 'Untitled'} ({shownCount(cat)})</option>
            {/each}
          </select>
        </label>
        <div class="spacer"></div>
        {#if activeCat && typeof selectedCat === 'number'}
          {@const ci = selectedCat}
          <button class="btn" onclick={() => addApp(activeCat!, ci)}><Plus size={15} /> Add app</button>
        {/if}
        <button class="btn" onclick={addCategory}><FolderPlus size={15} /> Add category</button>
      </div>
    {/if}

    <div class="view-body" bind:this={scroller}>
      <div class="page">
        {#if loading}
          <p class="muted">Loading…</p>
        {:else if loadError}
          <p class="err-text">{loadError}</p>
        {:else if file}
          {#if activeCat && typeof selectedCat === 'number'}
            {@const ci = selectedCat}
            <div class="cat-card">
              <label class="cat-field">
                <span class="label">Category title</span>
                <input class="text-input" placeholder="e.g. Browsers" bind:value={activeCat.title} />
              </label>
              <label class="cat-field">
                <span class="label">Id</span>
                <input class="text-input mono" placeholder="e.g. browsers" bind:value={activeCat.id} />
              </label>
              <button class="icon-button cat-menu" aria-label="Category actions" aria-haspopup="menu" onclick={(e) => categoryMenu(e, ci)}>
                <Ellipsis size={16} />
              </button>
            </div>
          {/if}

          {#if listed.length > 0}
            <ul class="apps">
              {#each listed as e (e.ci + '-' + e.ai)}
                <li class="app">
                  <button class="open" onclick={() => openEditor(e.ci, e.ai)}>
                    <AppIcon name={e.app.name || e.app.id || '?'} size={32} source={e.app.source} id={e.app.id} homepage={e.app.icon || e.app.homepage} />
                    <span class="text">
                      <span class="name">
                        {e.app.name || e.app.id || 'Untitled app'}
                        {#if e.app.custom}<span class="pill">Yours</span>{/if}
                      </span>
                      <span class="meta">
                        {[activeCat ? '' : e.cat.title || e.cat.id || 'Untitled', e.app.description ?? ''].filter(Boolean).join(' · ') ||
                          e.app.id ||
                          'No package id yet'}
                      </span>
                    </span>
                    {@render sourceBadges(e.app)}
                  </button>
                  <button
                    class="icon-button"
                    aria-label={`More actions for ${e.app.name || e.app.id || 'this app'}`}
                    aria-haspopup="menu"
                    onclick={(ev) => {
                      ev.stopPropagation();
                      openContextMenuAt(ev.currentTarget, appMenuItems(e.ci, e.ai));
                    }}
                  >
                    <Ellipsis size={16} />
                  </button>
                </li>
              {/each}
            </ul>
          {:else}
            <p class="empty muted">
              {filtering ? `Nothing matches “${filter}”.` : activeCat ? 'No apps in this category yet.' : 'No apps yet.'}
            </p>
          {/if}
          {#if !activeCat}
            <p class="hint">Pick a category above to add apps to it.</p>
          {/if}
        {/if}
      </div>
    </div>
  {/if}

  {#if dirty || saveError}
    <div class="savebar" role="region" aria-label="Unsaved changes">
      {#if saveError}
        <span class="err-text" role="alert">{saveError}</span>
      {:else}
        <span class="savebar-text">You have unsaved changes</span>
      {/if}
      <div class="spacer"></div>
      <button class="btn btn-ghost" onclick={discard} disabled={saving}>Discard</button>
      <button class="btn btn-tinted" onclick={save} disabled={saving || !dirty}>
        {saving ? 'Saving…' : 'Save'}
      </button>
    </div>
  {/if}
</div>

<style>
  .crumb {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    margin: 0 0 4px -2px;
    padding: 0;
    border: none;
    background: none;
    font: inherit;
    font-size: var(--fs-sm);
    color: var(--text-muted);
    cursor: pointer;
  }
  .crumb:hover {
    color: var(--text);
    text-decoration: none;
  }
  .title-row {
    display: flex;
    align-items: center;
    gap: 14px;
  }
  .toolbar {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 10px;
    max-width: 880px;
    padding: 0 24px 12px;
  }
  .filter {
    display: flex;
    align-items: center;
    gap: 8px;
    flex: 1;
    min-width: 200px;
    max-width: 320px;
    height: 36px;
    padding: 0 6px 0 10px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface);
    color: var(--text-muted);
  }
  .filter:focus-within {
    border-color: var(--accent);
    box-shadow: 0 0 0 1px var(--accent);
  }
  .filter input {
    flex: 1;
    min-width: 0;
    border: none;
    outline: none;
    background: transparent;
    color: var(--text);
    font: inherit;
    font-size: var(--fs-base);
  }
  .tiny {
    width: 24px;
    height: 24px;
  }
  .cat-pick {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: var(--fs-md);
  }
  .cat-pick select {
    width: auto;
    max-width: 240px;
  }
  .spacer {
    flex: 1;
  }
  .page {
    max-width: 880px;
    padding: 0 24px 32px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .hint {
    margin: 4px 0 0;
    font-size: var(--fs-sm);
    color: var(--text-muted);
  }
  .empty {
    margin: 12px 0;
  }
  .err-text {
    color: var(--danger);
    font-size: var(--fs-md);
  }
  .label {
    font-size: var(--fs-sm);
    font-weight: 500;
  }

  /* Category title/id when one category is picked. */
  .cat-card {
    display: flex;
    align-items: flex-end;
    gap: 12px;
    padding: 12px 12px 12px 16px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface-2);
  }
  .cat-field {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 5px;
    min-width: 0;
  }
  .cat-menu {
    flex-shrink: 0;
    margin-bottom: 2px;
  }

  /* The list */
  .apps {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .app {
    display: flex;
    align-items: center;
    gap: 4px;
    padding-right: 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface-2);
    transition: background 0.12s;
  }
  .app:hover {
    background: var(--surface-hover);
  }
  .open {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 14px;
    min-height: 58px;
    padding: 8px 10px 8px 14px;
    border: none;
    border-radius: var(--radius);
    background: transparent;
    color: var(--text);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .name {
    display: flex;
    align-items: center;
    gap: 8px;
    font-weight: 500;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .meta {
    font-size: var(--fs-sm);
    color: var(--text-muted);
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .pill {
    flex-shrink: 0;
    padding: 0 7px;
    border-radius: var(--radius-pill);
    background: color-mix(in srgb, var(--accent) 16%, transparent);
    color: color-mix(in srgb, var(--accent) 70%, var(--text));
    font-size: var(--fs-2xs);
    font-weight: 600;
    line-height: 1.7;
  }
  .src-dots {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .sd {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    border: 1px solid color-mix(in srgb, var(--c) 55%, transparent);
    border-radius: 50%;
    /* Pulled 30% toward --text, like accent text: >= 4.6:1 for every source. */
    color: color-mix(in srgb, var(--c) 70%, var(--text));
    font-size: 0.66rem;
    font-weight: 700;
    line-height: 1;
  }
  .sd-empty {
    width: 20px;
    height: 20px;
  }
  .sd.default {
    border: 1.5px solid var(--c);
    background: color-mix(in srgb, var(--c) 18%, transparent);
  }
  .sd.winget {
    --c: var(--winget);
  }
  .sd.scoop {
    --c: var(--scoop);
  }
  .sd.choco {
    --c: var(--choco);
  }
  .sd.msstore {
    --c: var(--msstore);
  }

  /* The editor: Settings-style groups of cards. */
  .group {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .group + .group {
    margin-top: 10px;
  }
  .group h2 {
    margin: 0 0 6px;
    font-size: var(--fs-base);
    font-weight: 600;
  }
  .field {
    width: 320px;
    max-width: 100%;
  }
  .chips-input {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 5px;
    min-height: 37px;
    padding: 4px 6px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface);
  }
  .chips-input:focus-within {
    border-color: var(--accent);
    box-shadow: 0 0 0 1px var(--accent);
  }
  .chips-input input {
    flex: 1;
    min-width: 70px;
    border: none;
    outline: none;
    background: transparent;
    color: var(--text);
    font: inherit;
    font-size: var(--fs-base);
    padding: 2px 4px;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    padding: 1px 3px 1px 9px;
    border-radius: var(--radius-pill);
    background: color-mix(in srgb, var(--text) 9%, transparent);
    font-size: var(--fs-sm);
  }
  .chip-x {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    padding: 0;
    border: none;
    border-radius: 50%;
    background: transparent;
    color: var(--text-muted);
  }
  .chip-x:hover {
    background: var(--surface-hover);
    color: var(--text);
  }
  .src-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .src-row {
    display: grid;
    grid-template-columns: 170px minmax(0, 1fr) 112px auto;
    align-items: center;
    gap: 8px;
  }
  .default-tag {
    justify-self: center;
    padding: 0 9px;
    border-radius: var(--radius-pill);
    background: color-mix(in srgb, var(--accent) 16%, transparent);
    color: color-mix(in srgb, var(--accent) 70%, var(--text));
    font-size: var(--fs-2xs);
    font-weight: 600;
    line-height: 1.8;
  }
  .small {
    padding: 5px 10px;
    font-size: var(--fs-sm);
  }
  .quiet {
    color: var(--text-muted);
  }
  .quiet:hover {
    color: var(--text);
  }
  .add-src {
    align-self: flex-start;
    margin: 8px 0 0 -10px;
  }

  /* Appears only with unsaved changes, like Windows Settings. */
  .savebar {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 24px;
    border-top: 1px solid var(--border);
    background: color-mix(in srgb, var(--accent) 8%, var(--surface-2));
  }
  .savebar-text {
    font-weight: 600;
    font-size: var(--fs-base);
  }
</style>

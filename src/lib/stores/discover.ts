import { goto } from '$app/navigation';
import type { LibSelection } from '$lib/installedGroups';

/** Shows every app with this tag (from a tag on an app's page). */
export function filterByTag(tag: string) {
  goto(`/?view=all&tag=${encodeURIComponent(tag)}`);
}

/** Jumps to a part of the app that used to live under "Library". */
export function openLibrary(section: LibSelection) {
  if (section === 'updates') goto('/updates');
  else if (section === 'activity') goto('/activity');
  else if (section === 'all') goto('/installed');
  else goto(`/installed?group=${encodeURIComponent(section)}`);
}

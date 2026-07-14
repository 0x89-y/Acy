import { enqueue, notice } from '$lib/stores/ops';
import { get } from 'svelte/store';
import * as api from '$lib/api';
import { confirmAction, confirmRemember } from '$lib/stores/confirm';
import { settings, setWarnChocoAdmin } from '$lib/stores/settings';
import type { Source } from '$lib/types';

/**
 * Chocolatey writes to C:\ProgramData and usually needs admin. We don't probe
 * elevation (that "check my own privileges" pattern trips antivirus heuristics);
 * instead we warn once before a choco write and let the user proceed. Returns
 * false only if they cancel.
 */
export async function ensureCanWrite(source: Source): Promise<boolean> {
  if (source !== 'choco') return true;
  if (!get(settings).warnChocoAdmin) return true;
  const { ok, remember } = await confirmRemember({
    title: 'Chocolatey may need administrator rights',
    message:
      'Chocolatey installs to a system folder and usually needs admin. If it fails, restart ' +
      'Acy as administrator (right-click Acy → Run as administrator), then try again.',
    confirmLabel: 'Continue',
    rememberLabel: "Don't remind me again"
  });
  if (ok && remember) setWarnChocoAdmin(false);
  return ok;
}

/**
 * Scoop apps live in "buckets" that must be added before their manifest
 * resolves - most curated apps are in `extras`, which isn't added by default.
 * If the needed bucket is missing, ask before adding it (rather than silently
 * changing the user's scoop setup), then add it. Returns false if they decline
 * or the add fails.
 */
async function ensureScoopBucket(id: string, name: string): Promise<boolean> {
  const bucket = await api.scoopNeededBucket(id);
  if (!bucket) return true;
  const ok = await confirmAction({
    title: `Add the "${bucket}" scoop bucket?`,
    message:
      `${name} is in scoop's "${bucket}" bucket, which isn't added yet. ` +
      `Acy can add it, then install.`,
    confirmLabel: 'Add bucket & install'
  });
  if (!ok) return false;
  return enqueue(`Add scoop bucket ${bucket}`, (opId) => api.addScoopBucket(bucket, opId));
}

/** Show a single summary toast after a batch install/uninstall/update. */
export function summarizeBatch(
  total: number,
  ok: number,
  verb: 'installed' | 'removed' | 'updated'
) {
  const fail = total - ok;
  if (fail === 0) notice(`${ok} ${ok === 1 ? 'app' : 'apps'} ${verb}.`, 'ok');
  else notice(`${ok} ${verb}, ${fail} failed.`, ok > 0 ? 'warn' : 'error');
}

// Shared install/update/uninstall action used by InstallButton and the
// multi-source split button, so the verify + activity-logging behaviour lives in
// one place.

export type InstallKind = 'install' | 'update' | 'uninstall';

export const VERBS: Record<InstallKind, string> = {
  install: 'Install',
  update: 'Update',
  uninstall: 'Uninstall'
};

/** The shell command to install a package from a given manager, or null (local). */
export function installCommand(source: Source, id: string): string | null {
  switch (source) {
    case 'winget':
      return `winget install --id ${id} -e`;
    case 'scoop':
      return `scoop install ${id}`;
    case 'choco':
      return `choco install ${id}`;
    case 'msstore':
      return `winget install --id ${id} --source msstore`;
    default:
      return null; // local has no command line
  }
}

/**
 * Queue an install/update/uninstall and resolve true on success. Trusts the
 * resulting installed/update state over the process exit code, since some
 * managers exit non-zero even when the change applied.
 */
export async function runOp(
  kind: InstallKind,
  source: Source,
  id: string,
  name = id
): Promise<boolean> {
  // Chocolatey writes need admin - bail out early with a clear message rather
  // than running and failing with a permissions error.
  if (!(await ensureCanWrite(source))) return false;

  // A scoop install may need its bucket added first - ask before doing so.
  if (source === 'scoop' && kind === 'install' && !(await ensureScoopBucket(id, name))) {
    return false;
  }

  // "local" runs an installer file: use the stored path, or ask for one. There
  // is nothing to verify afterwards, so trust the installer's exit code.
  if (source === 'local') {
    if (kind !== 'install') return false;
    let path = id;
    if (!path) {
      const picked = await api.pickInstaller();
      if (!picked) return false; // cancelled
      path = picked;
    }
    return enqueue(
      `Install ${name}`,
      (opId) => api.install('local', path, opId),
      undefined,
      undefined,
      { action: 'install', name, source }
    );
  }

  const idLower = id.toLowerCase();
  const verify = async (): Promise<boolean> => {
    if (kind === 'update') {
      const ups = await api.listUpdates([source]);
      return !ups.some((p) => p.id.toLowerCase() === idLower);
    }
    const inst = await api.listInstalled([source]);
    const present = inst.some((p) => p.id.toLowerCase() === idLower);
    return kind === 'uninstall' ? !present : present;
  };
  return enqueue(
    `${VERBS[kind]} ${name}`,
    (opId) => {
      if (kind === 'install') return api.install(source, id, opId);
      if (kind === 'update') return api.upgrade(source, id, opId);
      return api.uninstall(source, id, opId);
    },
    verify,
    undefined,
    { action: kind, name, source }
  );
}

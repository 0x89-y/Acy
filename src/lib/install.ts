import { enqueue } from '$lib/stores/ops';
import * as api from '$lib/api';
import type { Source } from '$lib/types';

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

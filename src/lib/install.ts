import { enqueue, notice } from '$lib/stores/ops';
import * as api from '$lib/api';
import type { Source } from '$lib/types';

export function summarizeBatch(
  total: number,
  ok: number,
  verb: 'installed' | 'removed' | 'updated'
) {
  const fail = total - ok;
  if (fail === 0) notice(`${ok} ${ok === 1 ? 'app' : 'apps'} ${verb}.`, 'ok');
  else notice(`${ok} ${verb}, ${fail} failed.`, ok > 0 ? 'warn' : 'error');
}

export type InstallKind = 'install' | 'update' | 'uninstall';

export const VERBS: Record<InstallKind, string> = {
  install: 'Install',
  update: 'Update',
  uninstall: 'Uninstall'
};

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
      return null;
  }
}

export async function runOp(
  kind: InstallKind,
  source: Source,
  id: string,
  name = id
): Promise<boolean> {
  if (source === 'local') {
    if (kind !== 'install') return false;
    let path = id;
    if (!path) {
      const picked = await api.pickInstaller();
      if (!picked) return false;
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

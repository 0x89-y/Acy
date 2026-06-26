import { writable, get } from 'svelte/store';
import { onOpLog } from '$lib/api';
import { logActivity, type ActivityAction } from './activity';
import type { Source } from '$lib/types';

export interface ActivityMeta {
  action: ActivityAction;
  name: string;
  source?: Source;
}

export type OpState = 'queued' | 'running' | 'done' | 'error';

export interface Op {
  id: string;
  title: string;
  state: OpState;
  lines: string[];
  code?: number;
  detail?: string;
}

interface Job {
  id: string;
  run: (opId: string) => Promise<number>;
  verify?: () => Promise<boolean>;
  resolve: (ok: boolean) => void;
  meta?: ActivityMeta;
}

export const ops = writable<Op[]>([]);

const queue: Job[] = [];
let working = false;
let listening = false;

const DISMISS_MS = 6000;

function updateOp(id: string, patch: (op: Op) => Op) {
  ops.update((list) => list.map((op) => (op.id === id ? patch(op) : op)));
}

export function dismiss(id: string) {
  ops.update((list) => list.filter((op) => op.id !== id));
}

async function ensureListener() {
  if (listening) return;
  listening = true;
  await onOpLog((log) => {
    updateOp(log.opId, (op) => ({ ...op, lines: [...op.lines, log.line] }));
  });
}

function newId(): string {
  if (typeof crypto !== 'undefined' && 'randomUUID' in crypto) {
    return crypto.randomUUID();
  }
  return `op-${Date.now()}-${Math.random().toString(16).slice(2)}`;
}

function scheduleDismiss(id: string) {
  setTimeout(() => {
    const op = get(ops).find((o) => o.id === id);
    if (op && op.state === 'done') dismiss(id);
  }, DISMISS_MS);
}

async function work() {
  if (working) return;
  working = true;
  while (queue.length > 0) {
    const job = queue.shift()!;
    updateOp(job.id, (op) => ({ ...op, state: 'running' }));

    let ok = false;
    try {
      const code = await job.run(job.id);
      ok = code === 0;
      if (!ok && job.verify) {
        try {
          ok = await job.verify();
        } catch {
        }
      }
      updateOp(job.id, (op) => {
        const lines =
          ok && code !== 0
            ? [...op.lines, `(exit code ${code}, but the change was applied)`]
            : op.lines;
        return { ...op, state: ok ? 'done' : 'error', code, lines };
      });
    } catch (e) {
      updateOp(job.id, (op) => ({ ...op, state: 'error', lines: [...op.lines, String(e)] }));
      ok = false;
    }

    if (job.meta) logActivity({ ...job.meta, ok });
    job.resolve(ok);
    if (ok) scheduleDismiss(job.id);
  }
  working = false;
}

export async function enqueue(
  title: string,
  run: (opId: string) => Promise<number>,
  verify?: () => Promise<boolean>,
  detail?: string,
  meta?: ActivityMeta
): Promise<boolean> {
  await ensureListener();
  const id = newId();
  ops.update((list) => [...list, { id, title, state: 'queued', lines: [], detail }]);
  return new Promise<boolean>((resolve) => {
    queue.push({ id, run, verify, resolve, meta });
    work();
  });
}

import { writable, get } from 'svelte/store';
import { onOpLog } from '$lib/api';

export type OpState = 'queued' | 'running' | 'done' | 'error';

export interface Op {
  id: string;
  title: string;
  state: OpState;
  lines: string[];
  code?: number;
  /** Optional context line, e.g. the list of apps an "update all" covers. */
  detail?: string;
}

interface Job {
  id: string;
  run: (opId: string) => Promise<number>;
  verify?: () => Promise<boolean>;
  resolve: (ok: boolean) => void;
}

/** All operations currently shown as toasts (queued, running, or finished). */
export const ops = writable<Op[]>([]);

const queue: Job[] = [];
let working = false;
let listening = false;

/** How long a successful toast lingers before auto-dismissing. */
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

// Process the queue one job at a time. Running installs sequentially avoids the
// Windows Installer "another installation is already in progress" conflict.
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
          // ok stays false
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

    job.resolve(ok);
    if (ok) scheduleDismiss(job.id);
  }
  working = false;
}

/**
 * Queue a streamed operation. It is shown immediately as a "Queued" toast and
 * runs once earlier operations finish. `run` receives the op id and returns the
 * process exit code; when that code is non-zero and `verify` is supplied, the
 * real state is checked and trusted. Resolves true on success.
 */
export async function enqueue(
  title: string,
  run: (opId: string) => Promise<number>,
  verify?: () => Promise<boolean>,
  detail?: string
): Promise<boolean> {
  await ensureListener();
  const id = newId();
  ops.update((list) => [...list, { id, title, state: 'queued', lines: [], detail }]);
  return new Promise<boolean>((resolve) => {
    queue.push({ id, run, verify, resolve });
    work();
  });
}

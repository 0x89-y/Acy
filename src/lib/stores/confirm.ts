import { writable } from 'svelte/store';

export interface ConfirmRequest {
  title: string;
  message?: string;
  confirmLabel?: string;
  cancelLabel?: string;
  danger?: boolean;
  alert?: boolean;
  rememberLabel?: string;
  resolve: (result: { ok: boolean; remember: boolean }) => void;
}

export const confirmRequest = writable<ConfirmRequest | null>(null);

function open(opts: Omit<ConfirmRequest, 'resolve'>): Promise<{ ok: boolean; remember: boolean }> {
  return new Promise((resolve) => {
    confirmRequest.set({ ...opts, resolve });
  });
}

export async function confirmAction(
  opts: Omit<ConfirmRequest, 'resolve' | 'rememberLabel'>
): Promise<boolean> {
  return (await open(opts)).ok;
}

export function confirmRemember(
  opts: Omit<ConfirmRequest, 'resolve'>
): Promise<{ ok: boolean; remember: boolean }> {
  return open(opts);
}

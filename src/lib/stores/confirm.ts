import { writable } from 'svelte/store';

// A tiny app-wide confirmation modal, imperatively awaited like `window.confirm`
// but rendered as a custom dialog (the native one is routed through the dialog
// plugin, which we don't grant, and is async — easy to misuse).
export interface ConfirmRequest {
  title: string;
  message?: string;
  confirmLabel?: string;
  cancelLabel?: string;
  /** Style the confirm button as destructive. */
  danger?: boolean;
  /** Message-only: hide the Cancel button (single acknowledge button). */
  alert?: boolean;
  /** When set, show a "don't ask again" checkbox with this label. */
  rememberLabel?: string;
  resolve: (result: { ok: boolean; remember: boolean }) => void;
}

export const confirmRequest = writable<ConfirmRequest | null>(null);

function open(opts: Omit<ConfirmRequest, 'resolve'>): Promise<{ ok: boolean; remember: boolean }> {
  return new Promise((resolve) => {
    confirmRequest.set({ ...opts, resolve });
  });
}

/** Show the confirm dialog and resolve to the user's yes/no choice. */
export async function confirmAction(
  opts: Omit<ConfirmRequest, 'resolve' | 'rememberLabel'>
): Promise<boolean> {
  return (await open(opts)).ok;
}

/** Confirm with a "don't ask again" checkbox; resolves both the choice and it. */
export function confirmRemember(
  opts: Omit<ConfirmRequest, 'resolve'>
): Promise<{ ok: boolean; remember: boolean }> {
  return open(opts);
}

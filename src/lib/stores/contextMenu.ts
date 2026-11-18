import { writable } from 'svelte/store';

export type CtxItem = {
  label: string;
  onSelect: () => void;
  danger?: boolean;
  disabled?: boolean;
};

export type CtxState = { x: number; y: number; items: CtxItem[] } | null;

export const contextMenu = writable<CtxState>(null);

export function openContextMenu(e: MouseEvent, items: CtxItem[]) {
  e.preventDefault();
  e.stopPropagation();
  const w = 190;
  const h = 8 + items.length * 36;
  const x = Math.max(8, Math.min(e.clientX, window.innerWidth - w - 8));
  const y = Math.max(8, Math.min(e.clientY, window.innerHeight - h - 8));
  contextMenu.set({ x, y, items });
}

/** Opens the menu below an element (a "⋯" button), for mouse and keyboard alike. */
export function openContextMenuAt(anchor: HTMLElement, items: CtxItem[]) {
  const r = anchor.getBoundingClientRect();
  const w = 190;
  const h = 8 + items.length * 36;
  const x = Math.max(8, Math.min(r.right - w, window.innerWidth - w - 8));
  const below = r.bottom + 4;
  const y = below + h > window.innerHeight - 8 ? Math.max(8, r.top - h - 4) : below;
  contextMenu.set({ x, y, items });
}

export function closeContextMenu() {
  contextMenu.set(null);
}

import { writable } from 'svelte/store';

export type CtxItem = {
  label: string;
  onSelect: () => void;
  danger?: boolean;
  disabled?: boolean;
};

export type CtxState = { x: number; y: number; items: CtxItem[] } | null;

/** The single app-wide right-click menu (rendered once in the layout). */
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

export function closeContextMenu() {
  contextMenu.set(null);
}

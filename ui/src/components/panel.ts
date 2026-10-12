// V103-05, V103-06 What a panel inside `PanelLayout` can do right now, handed to its frame
// (`PanelCard`) through context so each card needn't pass it on.

import { getContext, setContext } from 'svelte';

export interface PanelCtl {
  /** Unlocked: drags the panel by its title bar. */
  grab?: (e: PointerEvent) => void;
  /** Unlocked: closes it into the closed-panels corner. */
  close?: () => void;
  unlocked: boolean;
}

/** A closed panel in the corner: its name and a few words on what it is. */
export interface TrayItem {
  id: string;
  title: string;
  hint: string;
}

const KEY = Symbol('panel');

export function setPanel(get: () => PanelCtl): void {
  setContext(KEY, get);
}

export function usePanel(): (() => PanelCtl) | undefined {
  return getContext<(() => PanelCtl) | undefined>(KEY);
}

/** Starts a panel drag from a title bar unless the press was on a control in it. */
export function grabFrom(grab: ((e: PointerEvent) => void) | undefined): ((e: PointerEvent) => void) | undefined {
  if (!grab) return undefined;
  return (e) => {
    if ((e.target as HTMLElement).closest('button, input, select, textarea, a, [role="slider"]')) return;
    grab(e);
  };
}

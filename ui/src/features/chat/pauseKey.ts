// C5, V108-C04 The pause key: held only while the modifier itself is down on its own.
// Typing with it (Shift+letter, Ctrl+V) doesn't pause, and neither does AltGr, which Windows
// sends as Ctrl+Alt: on a German layout @ € { } \ | are all AltGr. In a text field only Alt
// pauses: Shift before a capital or Ctrl for a shortcut would flicker a pause while typing.

import type { PauseKey } from '../../ipc/bindings/PauseKey';

/** The key event fields this reads. */
export interface KeyLike {
  type: string;
  key: string;
  altKey: boolean;
  ctrlKey: boolean;
  shiftKey: boolean;
  metaKey: boolean;
  getModifierState(key: string): boolean;
}

const NAMES = { ctrl: 'Control', alt: 'Alt', shift: 'Shift' } as const;

/** Keys that only change others: pressing one isn't typing. */
export const MODIFIERS = new Set(['Control', 'Alt', 'AltGraph', 'Shift', 'Meta', 'CapsLock', 'OS']);

/** Whether `el` takes typed text (the message box, search, a settings field). */
export function editable(el: unknown): boolean {
  const e = el as { isContentEditable?: boolean; tagName?: string } | null;
  if (!e) return false;
  if (e.isContentEditable) return true;
  return e.tagName === 'INPUT' || e.tagName === 'TEXTAREA' || e.tagName === 'SELECT';
}

/** Whether the pause key is held after this keydown or keyup. Any other key lets go of it. */
export function pauseHeld(key: PauseKey, e: KeyLike, inField: boolean): boolean {
  if (key === 'none' || e.type !== 'keydown' || (inField && key !== 'alt')) return false;
  if (e.key !== NAMES[key] || e.getModifierState('AltGraph')) return false;
  // Alone: no other modifier with it (AltGr also shows up as Ctrl+Alt).
  const others = [e.ctrlKey && key !== 'ctrl', e.altKey && key !== 'alt', e.shiftKey && key !== 'shift', e.metaKey];
  return !others.some(Boolean);
}

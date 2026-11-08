// V110-M20 What clicking a name does: Settings → Chat picks an action for a click, a middle
// click, Ctrl+click and Shift+click. Shift+right-click replying (V110-C01) is separate.

import type { NameClick } from '../../ipc/bindings/NameClick';
import type { NameClicks } from '../../ipc/bindings/NameClicks';

export type Trigger = keyof NameClicks;

/** Which binding a pointer event uses; null for Alt or Meta clicks, which keep the usercard. */
export function triggerOf(e: Pick<MouseEvent, 'button' | 'ctrlKey' | 'shiftKey' | 'altKey' | 'metaKey'>): Trigger | null {
  if (e.altKey || e.metaKey) return null;
  if (e.button === 1) return 'middle';
  if (e.button !== 0) return null;
  if (e.ctrlKey) return 'ctrl';
  if (e.shiftKey) return 'shift';
  return 'plain';
}

/** What can be done with this name, here. */
export interface Can {
  /** The name is the sender of the line it is on, and replying is allowed. */
  reply: boolean;
  /** The message box takes a mention. */
  mention: boolean;
  /** This account may time out and ban them here. */
  mod: boolean;
}

/** The action that really runs: what can't happen here (a mod action where you aren't a mod, a
 *  reply to someone who didn't write the line) opens the usercard instead. */
export function actionFor(bound: NameClick, can: Can): NameClick {
  switch (bound) {
    case 'reply':
      return can.reply ? bound : 'usercard';
    case 'mention':
      return can.mention ? bound : 'usercard';
    case 'timeout':
    case 'ban':
      return can.mod ? bound : 'usercard';
    default:
      return bound;
  }
}

/** The shortest quick timeout, or Twitch's ten minutes when none is set. */
export function defaultTimeout(presets: readonly number[]): number {
  return presets.length ? Math.min(...presets) : 600;
}

// V110-A11 Reader mode: the chat log takes Tab and a polite live region reads out the newest
// lines, a few at a time. Off by default. This viewer's storage (like the other per-viewer
// switches); lost storage means off.

import type { ChatMessage } from '../../ipc/bindings/ChatMessage';
import { lineLabel } from './lineSelect.ts';

export const READER_KEY = 'cyex.chatReader';
/** Longest wait between two readouts. */
export const READER_MS = 2500;
/** Most lines in one readout; the rest are skipped, newest kept. */
export const READER_LINES = 3;
/** Longest readout, in characters. */
export const READER_CHARS = 400;

export function readReader(store: Pick<Storage, 'getItem'> | null = safe()): boolean {
  try {
    return store?.getItem(READER_KEY) === '1';
  } catch {
    return false;
  }
}

export function writeReader(on: boolean, store: Pick<Storage, 'setItem' | 'removeItem'> | null = safe()): void {
  try {
    if (on) store?.setItem(READER_KEY, '1');
    else store?.removeItem(READER_KEY);
  } catch {
    // Blocked storage: the choice lasts until the window closes.
  }
}

function safe(): Storage | null {
  try {
    return typeof localStorage === 'undefined' ? null : localStorage;
  } catch {
    return null;
  }
}

/** What to read out now: the lines after `last` (the newest READER_LINES of them), and the id to
 *  carry on from. Deleted lines are not read.
 *  `last` null or gone from the list: only the newest line. */
export function readout(list: readonly ChatMessage[], last: string | null): { text: string; last: string | null } {
  const tip = list.at(-1)?.id ?? last;
  const at = last === null ? -1 : list.findIndex((m) => m.id === last);
  const fresh = (at === -1 ? list.slice(-1) : list.slice(at + 1)).filter((m) => !m.flags.includes('deleted'));
  const text = fresh.slice(-READER_LINES).map(lineLabel).filter(Boolean).join('. ');
  return { text: text.length > READER_CHARS ? `${text.slice(0, READER_CHARS - 1)}…` : text, last: tip };
}

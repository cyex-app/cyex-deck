// V108-F54 Keyboard line selection: which line Alt+Up / Alt+Down lands on, how far to scroll
// to show it with its button bar, which key does what, and how a screen reader hears it.
// Pure, so the list only wires it to the DOM.

import { plainText } from './text.ts';
import type { ChatMessage } from '../../ipc/bindings/ChatMessage';

/** The line after a step. `null` = nothing selected: Alt+Down past the newest line, or Alt+Down
 *  with nothing selected. Alt+Up with nothing selected (or a line that is gone) takes the
 *  newest; at the oldest drawn line it stays. */
export function stepLine(ids: readonly string[], current: string | null, dir: -1 | 1): string | null {
  if (!ids.length) return null;
  const at = current === null ? -1 : ids.indexOf(current);
  if (at < 0) return dir < 0 ? ids[ids.length - 1] : null;
  const next = at + dir;
  if (next < 0) return ids[0];
  return next >= ids.length ? null : ids[next];
}

/** How far to scroll (positive = down) so a line shows whole, with `pad` px above it for the
 *  button bar that hangs over its top edge. 0 = already in view. Tall lines show their top. */
export function revealDelta(line: { top: number; bottom: number }, box: { top: number; bottom: number }, pad: number): number {
  const above = line.top - (box.top + pad);
  if (above < 0) return above;
  const below = line.bottom - box.bottom;
  if (below <= 0) return 0;
  // Taller than the box: the top wins, the rest scrolls into view by the reader.
  return Math.min(below, above);
}

export type LineKey = 'reply' | 'pin' | 'menu' | 'clear' | 'up' | 'down';

/** What a key does while a line is selected. Alt+Up and Alt+Down are the rebindable hotkeys,
 *  so they are not here. Null = the key is not ours. */
export function lineKey(e: { key: string; altKey: boolean; ctrlKey: boolean; metaKey: boolean; shiftKey: boolean }): LineKey | null {
  if (e.ctrlKey || e.metaKey) return null;
  if (e.key === 'ContextMenu' || (e.shiftKey && e.key === 'F10')) return 'menu';
  if (e.altKey || e.shiftKey) return null;
  switch (e.key) {
    case 'Escape':
      return 'clear';
    case 'Enter':
    case 'r':
    case 'R':
      return 'reply';
    case 'p':
    case 'P':
      return 'pin';
    case 'm':
    case 'M':
      return 'menu';
    default:
      return null;
  }
}

/** What a screen reader says for the selected line: "name: text", or the notice's own text. */
export function lineLabel(m: Pick<ChatMessage, 'sender' | 'body' | 'notice'>): string {
  const text = plainText(m.body) || m.notice?.text || '';
  return m.sender ? `${m.sender.display}: ${text}` : text;
}

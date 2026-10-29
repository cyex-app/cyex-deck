// V108-N09 What a key does while suggestions are open. Pure, so the list and the row share
// one rule set and it can be tested.

export type SuggestStyle = 'list' | 'row';

export type SuggestKey =
  | { do: 'move'; index: number }
  | { do: 'pick' }
  | { do: 'close' };

export interface KeyInfo {
  key: string;
  shift?: boolean;
  ctrl?: boolean;
}

/** `null`: the key is not for the suggestions (the box handles it). List: Up/Down move, Tab
 * and Enter pick. Row: Tab, Shift+Tab, Left and Right move (they wrap), Enter picks. In both,
 * Esc closes. `arg`: Enter sends what is typed, only a click or Tab fills (V1051-05). */
export function suggestKey(style: SuggestStyle, k: KeyInfo, index: number, count: number, arg = false): SuggestKey | null {
  if (count <= 0) return null;
  const step = (d: number): SuggestKey => ({ do: 'move', index: (((index + d) % count) + count) % count });
  if (k.key === 'Escape') return { do: 'close' };
  if (style === 'row') {
    if (k.key === 'ArrowRight') return step(1);
    if (k.key === 'ArrowLeft') return step(-1);
    if (k.key === 'Tab') return step(k.shift ? -1 : 1);
    if (k.key === 'Enter' && !k.ctrl && !arg) return { do: 'pick' };
    // A command or argument row: Tab walked it, Enter sends. Nothing else picks.
    return null;
  }
  if (k.key === 'ArrowDown') return step(1);
  if (k.key === 'ArrowUp') return step(-1);
  if (k.key === 'Tab' || (k.key === 'Enter' && !k.ctrl && !arg)) return { do: 'pick' };
  return null;
}

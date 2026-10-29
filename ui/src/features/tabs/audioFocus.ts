// V108-N13 Multi-view: one tile has the sound. Pure rules for who it is; the state is in
// `audioFocus.svelte.ts`.

export interface FocusInput {
  /** The tab's tiles that play a stream, in tab order. */
  players: readonly string[];
  /** The tile the user last picked in this tab, if any. */
  claimed: string | null;
  /** Settings → Appearance: one tile has sound. */
  enabled: boolean;
  /** V108-N14 The tile in theatre, if it is in this tab. */
  theatre: string | null;
}

/** The tile that is heard, or null when every tile keeps its own sound. */
export function heardTile(i: FocusInput): string | null {
  if (i.theatre && i.players.includes(i.theatre)) return i.theatre;
  if (!i.enabled || i.players.length < 2) return null;
  return i.claimed && i.players.includes(i.claimed) ? i.claimed : i.players[0];
}

/** Whether `tile` is silenced by the focus (its own mute and volume stay as they are). */
export function hushedByFocus(tile: string, i: FocusInput): boolean {
  if (!i.players.includes(tile)) return false;
  const heard = heardTile(i);
  return heard !== null && heard !== tile;
}

/** A click on `target` inside a tile moves the sound, unless it is in a place for typing. */
export function claimsOnClick(target: { tagName?: string; isContentEditable?: boolean } | null): boolean {
  if (!target) return true;
  if (target.isContentEditable) return false;
  const tag = (target.tagName ?? '').toUpperCase();
  return tag !== 'INPUT' && tag !== 'TEXTAREA' && tag !== 'SELECT';
}

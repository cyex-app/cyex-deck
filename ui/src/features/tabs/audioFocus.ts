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
  /** The players whose channel is live; left out = all count as live. */
  live?: readonly string[];
}

/** The tile that is heard, or null when every tile keeps its own sound. */
export function heardTile(i: FocusInput): string | null {
  if (i.theatre && i.players.includes(i.theatre)) return i.theatre;
  if (!i.enabled || i.players.length < 2) return null;
  if (i.claimed && i.players.includes(i.claimed)) return i.claimed;
  // Not picked: the first tile with a stream on air, else the first (all offline).
  return i.players.find((p) => !i.live || i.live.includes(p)) ?? i.players[0];
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

/** V108-N13 A click that takes the sound also unmutes a tile the user had muted. Not when the
 * focus left it alone (it already had the sound, so a mute stays) or the click was on one of the
 * player's own buttons or sliders (its volume button decides for itself). */
export function unmutesOnClaim(i: { silencedByFocus: boolean; tileMuted: boolean; onControl: boolean }): boolean {
  return i.silencedByFocus && i.tileMuted && !i.onControl;
}

/** A click landed on a button or slider (the target or something around it). */
export function isControl(target: { closest?: (s: string) => unknown } | null): boolean {
  return !!target?.closest?.('button, [role="slider"], [role="menuitem"], a, input, select');
}

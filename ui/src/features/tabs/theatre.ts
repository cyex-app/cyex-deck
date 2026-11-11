// V108-N14 Theatre mode: one tile fills the tab with its chat beside it. Pure rules.

export interface TheatreTile {
  id: string;
  /** Plays a stream (not a feed, not chat-only). */
  player: boolean;
}

/** The tile to put in theatre: the one with the sound if it has a stream, else the first with one. */
export function theatreTarget(tiles: readonly TheatreTile[], heard: string | null): string | null {
  const players = tiles.filter((t) => t.player);
  return players.find((t) => t.id === heard)?.id ?? players[0]?.id ?? null;
}

/** Alt+T: leaves when in theatre, else enters with `target` (nothing to enter = stays out). */
export function toggled(current: string | null, target: string | null): string | null {
  return current ? null : target;
}

/** Theatre stays only while its tile is in the shown tab. */
export function stillHere(current: string | null, shownTiles: readonly string[]): string | null {
  return current && shownTiles.includes(current) ? current : null;
}

export interface EscapeState {
  key: string;
  composing: boolean;
  /** A layer (menu, dialog) took the key. */
  taken: boolean;
  fullscreen: boolean;
  /** ms since a player's fullscreen ended. */
  sinceFullscreen: number;
}

/** Esc leaves theatre only when nothing else wanted it: a layer, or the fullscreen it ends first. */
export function escLeaves(e: EscapeState, current: string | null): boolean {
  if (!current || e.key !== 'Escape' || e.composing || e.taken || e.fullscreen) return false;
  return e.sinceFullscreen > 300;
}

/** V110-B2 The stream page header (avatar, name, Stream / About / Clips / Videos) hides in theatre,
 *  so the player takes the full height. Kept when the tile's chat isn't there to carry its buttons. */
export function headerHidden(current: string | null, tile: string, chatCarriesButtons: boolean): boolean {
  return current !== null && current === tile && chatCarriesButtons;
}

/** A tile's rect in the tab while in theatre: the whole tab. */
export function fullBox(w: number, h: number): { x: number; y: number; w: number; h: number } {
  return { x: 0, y: 0, w, h };
}

/** V110-B10 The chat is folded away in this tile for the length of theatre. */
export function chatHiddenIn(current: string | null, tile: string, hidden: boolean): boolean {
  return current !== null && current === tile && hidden;
}

/** V110-B10 The button in the player: only in theatre, for the theatre tile, with a chat to hide
 *  (or one hidden by this button). Its label says what a click does. */
export function chatToggle(current: string | null, tile: string, chatOn: boolean, hidden: boolean): 'Hide chat' | 'Show chat' | null {
  if (current === null || current !== tile || !chatOn) return null;
  return hidden ? 'Show chat' : 'Hide chat';
}

/** V110-B11 How long the pointer may rest on the player before the Hide chat / Show chat button fades out. */
export const REST_MS = 1000;

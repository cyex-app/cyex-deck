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

/** A tile's rect in the tab while in theatre: the whole tab. */
export function fullBox(w: number, h: number): { x: number; y: number; w: number; h: number } {
  return { x: 0, y: 0, w, h };
}

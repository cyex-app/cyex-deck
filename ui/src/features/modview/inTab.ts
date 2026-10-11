// V102-26 A mod view opened from a tab lives in that tab, in the tile of its channel. With
// no room there it goes out into its own window, and closing that window brings it back
// (V102-16). The rules, apart from the store that keeps them (`where.svelte.ts`).

/** One tile's mod view. */
export interface InTile {
  login: string;
  /** In its own window now, for room or by hand. */
  out: boolean;
  /** Its window was seen open, so the window closing brings it back. */
  seen: boolean;
  /** Just back from its window: still no room closes it instead of popping it out again. */
  back: boolean;
}

export const opened = (login: string): InTile => ({ login, out: false, seen: false, back: false });

export type Step = 'show' | 'popout' | 'close';

/** What a tile does once its mod view is fitted: shows it, pops it out for room, or, just
 * back from its window with still no room, closes it so it doesn't bounce. */
export function step(out: boolean, back: boolean): Step {
  if (!out) return 'show';
  return back ? 'close' : 'popout';
}

/** The open pop-outs changed: a mod view whose window opened is marked seen, one whose
 * window closed comes back into its tile. `null` when nothing changed. */
export function docked(tiles: Map<string, InTile>, routes: string[], route: (login: string) => string): Map<string, InTile> | null {
  let next: Map<string, InTile> | null = null;
  for (const [id, t] of tiles) {
    if (!t.out) continue;
    const open = routes.includes(route(t.login));
    if (open === t.seen) continue;
    next ??= new Map(tiles);
    next.set(id, open ? { ...t, seen: true } : { ...t, out: false, seen: false, back: true });
  }
  return next;
}

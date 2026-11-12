// V110-N7 F5 reloads the shown tab (its chats rejoin, its players reload), Ctrl+R every tab.
// Pure picking and a small registry, so the rules are testable without the app.

export interface Reloadable {
  /** The tile it belongs to; one without a tile (stream manager, mod view) is never reloaded here. */
  tile?: string;
  run(): void;
}

/** What a chat or player in a tile registers: removed again by the returned function. */
export class ReloadRegistry {
  private items = new Set<Reloadable>();
  add(item: Reloadable): () => void {
    this.items.add(item);
    return () => void this.items.delete(item);
  }
  list(): Reloadable[] {
    return [...this.items];
  }
}

export const chatReloads = new ReloadRegistry();
export const playerReloads = new ReloadRegistry();

export interface ReloadTab {
  id: string;
  tiles: { id: string; feed?: unknown }[];
}

/** The chats and players a reload runs. `tab` = only that tab (none: nothing), `all` = every tab.
 *  Feed tiles have no chat or player of their own, so a feed tab picks nothing. */
export function reloadPicks(o: { scope: 'tab' | 'all'; shown?: ReloadTab; tabs: ReloadTab[]; chats: Reloadable[]; players: Reloadable[] }): {
  chats: Reloadable[];
  players: Reloadable[];
} {
  const from = o.scope === 'tab' ? (o.shown ? [o.shown] : []) : o.tabs;
  const ids = new Set(from.flatMap((t) => t.tiles.filter((x) => !x.feed).map((x) => x.id)));
  const keep = (list: Reloadable[]) => list.filter((x) => x.tile !== undefined && ids.has(x.tile));
  return { chats: keep(o.chats), players: keep(o.players) };
}

/** Ignores a second press within `ms` of the one that ran. */
export class PressGate {
  private last = -Infinity;
  private ms: number;
  constructor(ms = 2000) {
    this.ms = ms;
  }
  allow(now: number): boolean {
    if (now - this.last < this.ms) return false;
    this.last = now;
    return true;
  }
}

/** V110-N7 Ctrl+R in the main window tells every pop-out to reload its own chats and players. */
export const RELOAD_ALL_EVENT = 'cyex:reload-all';

/** Runs every registered chat and player of this window (a pop-out has no tabs list to filter by). */
export function reloadEverything(): void {
  for (const c of chatReloads.list()) c.run();
  for (const p of playerReloads.list()) p.run();
}

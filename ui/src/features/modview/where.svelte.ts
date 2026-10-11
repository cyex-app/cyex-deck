// 1.46, 1.47, V102-16 Where a channel's mod view shows. V102-26 The shield in a tab opens
// it inside that tab, in the tile of its channel (`openIn`): `fitModview` in lib/fit.ts
// places it beside the stream, in the chat's place when there's no room, else in its own
// window, which brings it back into the tile when it closes. Not saved: every tab starts
// without it. Elsewhere in the main window (the stream manager's chat) it opens as a view
// next to the tabs and pops out from there; from a chat in another window (a popped-out
// chat, a `#channel` chat) it opens in a window of its own.

import { popOut } from '../../app/panels';
import { windowsIpc } from '../../ipc/windows';
import { currentWindow } from '../../lib/window.svelte';
import { tabs } from '../tabs/tabs.svelte';
import { docked, type InTile, opened, step } from './inTab';

export const modviewRoute = (login: string) => `modview/${login}`;
const title = (login: string) => `Mod view · ${login}`;

class ModViewPlace {
  /** The channel the main window's mod view is for. */
  login = $state<string | null>(null);
  /** Goes up on every `open`, so the main window switches to the view. */
  asks = $state(0);
  /** V102-26 Tile id → its channel's mod view inside the tab. */
  tiles = $state.raw<Map<string, InTile>>(new Map());
  private listening = false;

  /** Not from a tab: the main window's view, or a window of its own. */
  open(login: string): void {
    if (currentWindow.isMain) {
      this.login = login;
      this.asks++;
    } else {
      void popOut(modviewRoute(login), title(login));
    }
  }

  /** V102-26 Open in the tile (its window, if it's out). */
  isOpen(tile: string): boolean {
    return this.tiles.has(tile);
  }

  /** V102-26 Shown inside the tile now. */
  isIn(tile: string): boolean {
    return this.tiles.get(tile)?.out === false;
  }

  /** V102-26 The shield in a tile: opens the mod view there, or closes it. Out in its own
   * window, that window comes to the front. */
  toggleIn(tile: string, login: string): void {
    const t = this.tiles.get(tile);
    if (!t) this.set(tile, opened(login));
    else if (t.out) void popOut(modviewRoute(t.login), title(t.login));
    else this.close(tile);
  }

  close(tile: string): void {
    if (!this.tiles.has(tile)) return;
    const next = new Map(this.tiles);
    next.delete(tile);
    this.tiles = next;
  }

  /** V102-26 The tile placed its mod view: `out` when there's no room for it. */
  fitted(tile: string, out: boolean): void {
    const t = this.tiles.get(tile);
    if (!t || t.out) return;
    const s = step(out, t.back);
    if (s === 'popout') this.popOut(tile);
    else if (s === 'close') this.close(tile);
    else if (t.back) this.set(tile, { ...t, back: false });
  }

  /** V102-26 Into its own window, for room or by hand; closing it brings it back here. */
  popOut(tile: string): void {
    const t = this.tiles.get(tile);
    if (!t) return;
    this.listen();
    this.set(tile, { ...t, out: true, seen: false, back: false });
    void popOut(modviewRoute(t.login), title(t.login)).then(async (ok) => {
      if (!ok) return this.close(tile);
      // Its window may have been open already (no change to hear of).
      this.sync(await windowsIpc.panels().catch(() => []));
    });
  }

  private set(tile: string, t: InTile): void {
    this.tiles = new Map(this.tiles).set(tile, t);
  }

  private listen(): void {
    if (this.listening) return;
    this.listening = true;
    void windowsIpc.onPanels((routes) => this.sync(routes));
  }

  private sync(routes: string[]): void {
    const next = docked(this.tiles, routes, modviewRoute);
    if (!next) return;
    // A tile closed meanwhile has nowhere to come back to.
    for (const [id, t] of next) if (!t.out && t.back && !tabs.tabOf(id)) next.delete(id);
    this.tiles = next;
  }
}

export const modviewPlace = new ModViewPlace();

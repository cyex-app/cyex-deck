// Channel tabs (spec 02). Rust owns the list (the `tabs` settings partition) and every
// edit goes through `apply`. This adds what isn't stored: live status (1.29) and which
// tabs are popped out (1.51, 3.14).

import { popOut } from '../../app/panels';
import type { ChannelInfo } from '../../ipc/bindings/ChannelInfo';
import type { Tab } from '../../ipc/bindings/Tab';
import type { TabGroup } from '../../ipc/bindings/TabGroup';
import type { TabOp } from '../../ipc/bindings/TabOp';
import type { Tile } from '../../ipc/bindings/Tile';
import type { TileShow } from '../../ipc/bindings/TileShow';
import { tabsIpc } from '../../ipc/tabs';
import { windowsIpc } from '../../ipc/windows';
import { settings } from '../../lib/settings.svelte';
import { toast } from '../../lib/toast.svelte';

export type LiveState = 'live' | 'rerun' | null;

const TAB_ROUTE = 'tab/';

export const tabRoute = (id: string) => `${TAB_ROUTE}${id}`;

class TabsStore {
  /** Last live check, by channel id. */
  live = $state.raw<Record<string, ChannelInfo>>({});
  /** Tabs shown in their own window. */
  popped = $state.raw<Set<string>>(new Set());

  get list(): Tab[] {
    return settings.v.tabs?.tabs ?? [];
  }

  get groups(): TabGroup[] {
    return settings.v.tabs?.groups ?? [];
  }

  get active(): string | null {
    return settings.v.tabs?.active ?? null;
  }

  get lastShow(): TileShow {
    return settings.v.tabs?.lastShow ?? { player: true, chat: true };
  }

  tab(id: string | null | undefined): Tab | undefined {
    return id ? this.list.find((t) => t.id === id) : undefined;
  }

  group(id: string | null | undefined): TabGroup | undefined {
    return id ? this.groups.find((g) => g.id === id) : undefined;
  }

  /** The tab holding a tile. */
  tabOf(tileId: string): Tab | undefined {
    return this.list.find((t) => tiles(t).some((x) => x.id === tileId));
  }

  liveState(tab: Tab): LiveState {
    let state: LiveState = null;
    for (const tile of tiles(tab)) {
      const stream = this.live[tile.channel.id]?.stream;
      if (stream && !stream.rerun) return 'live';
      if (stream) state = 'rerun';
    }
    return state;
  }

  async load(): Promise<void> {
    const [live, panels] = await Promise.all([tabsIpc.live(), windowsIpc.panels()]);
    this.setLive(live);
    this.setPopped(panels);
    await tabsIpc.onLive((l) => this.setLive(l));
    await windowsIpc.onPanels((p) => this.setPopped(p));
  }

  /** Applies an edit. Resolves with the id of a new tab or group. */
  async apply(op: TabOp): Promise<string | null> {
    try {
      const applied = await tabsIpc.apply(op);
      settings.v.tabs = applied.tabs;
      return applied.made;
    } catch (e) {
      toast.error(String(e));
      return null;
    }
  }

  activate(id: string | null): void {
    if (id !== this.active) void this.apply({ op: 'activate', tab: id });
  }

  close(ids: string[]): void {
    if (ids.length) void this.apply({ op: 'close', tabs: ids });
  }

  /** 1.51 Opens the tab in its own window; the main window shows where it went. */
  popOut(tab: Tab): void {
    void popOut(tabRoute(tab.id), label(tab));
  }

  /** 3.14 Closes the tab's own window; it's back in the main window. */
  dock(id: string): void {
    windowsIpc.closePanel(tabRoute(id)).catch((e) => toast.error(String(e)));
  }

  private setLive(list: ChannelInfo[]): void {
    this.live = Object.fromEntries(list.map((c) => [c.id, c]));
  }

  private setPopped(routes: string[]): void {
    this.popped = new Set(routes.filter((r) => r.startsWith(TAB_ROUTE)).map((r) => r.slice(TAB_ROUTE.length)));
  }
}

export const tabs = new TabsStore();

export function tiles(tab: Tab): Tile[] {
  return tab.columns.flatMap((c) => c.tiles);
}

/** 1.22 The alias, or the channel names. */
export function label(tab: Tab): string {
  return tab.alias ?? tiles(tab).map((t) => t.channel.name).join(', ');
}

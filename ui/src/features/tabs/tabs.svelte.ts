// Channel tabs (spec 02). Rust owns the list (the `tabs` settings partition) and every
// edit goes through `apply`. This adds what isn't stored: live status (1.29), which tabs
// are popped out (1.51, 3.14), unread chats (U9) and whether a close can be undone (U10).

import { popOut } from '../../app/panels';
import type { ChannelInfo } from '../../ipc/bindings/ChannelInfo';
import type { RoomUnread } from '../../ipc/bindings/RoomUnread';
import type { Tab } from '../../ipc/bindings/Tab';
import type { TabGroup } from '../../ipc/bindings/TabGroup';
import type { TabOp } from '../../ipc/bindings/TabOp';
import type { Tile } from '../../ipc/bindings/Tile';
import type { TileShow } from '../../ipc/bindings/TileShow';
import type { Unread } from '../../ipc/bindings/Unread';
import { tabsIpc } from '../../ipc/tabs';
import { windowsIpc } from '../../ipc/windows';
import { accounts } from '../../lib/accounts.svelte';
import { settings } from '../../lib/settings.svelte';
import { toast } from '../../lib/toast.svelte';

export type LiveState = 'live' | 'rerun' | null;
export type UnreadState = 'message' | 'mention' | null;

const TAB_ROUTE = 'tab/';
const CHAT_ROUTE = 'chat/';

export const tabRoute = (id: string) => `${TAB_ROUTE}${id}`;
/** 1.58 A tile's chat on its own. */
export const chatRoute = (tileId: string) => `${CHAT_ROUTE}${tileId}`;

class TabsStore {
  /** Last live check, by channel id. */
  live = $state.raw<Record<string, ChannelInfo>>({});
  /** Tabs shown in their own window. */
  popped = $state.raw<Set<string>>(new Set());
  /** 1.58 Tiles whose chat is in its own window. */
  chats = $state.raw<Set<string>>(new Set());
  /** U9 Chats with news, by `account/login`. */
  unread = $state.raw<Record<string, Unread>>({});
  /** U10 Something closed can come back. */
  reopenable = $state(false);

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

  /** U9 How lit the tab is, per Settings → Appearance. */
  unreadState(tab: Tab): UnreadState {
    const mode = settings.v.appearance.tabUnread;
    if (mode === 'off') return null;
    let state: UnreadState = null;
    for (const tile of tiles(tab)) {
      if (!tile.show.chat) continue;
      const level = this.unread[roomKey(tile.account, tile.channel.login)];
      if (level === 'mention') return 'mention';
      if (level === 'message' && mode === 'all') state = 'message';
    }
    return state;
  }

  async load(): Promise<void> {
    const [live, panels, unread, reopenable] = await Promise.all([
      tabsIpc.live(),
      windowsIpc.panels(),
      tabsIpc.unread(),
      tabsIpc.reopenable(),
    ]);
    this.setLive(live);
    this.setPopped(panels);
    this.setUnread(unread, true);
    this.reopenable = reopenable;
    await tabsIpc.onLive((l) => this.setLive(l));
    await windowsIpc.onPanels((p) => this.setPopped(p));
    await tabsIpc.onUnread((u) => this.setUnread(u, false));
    await tabsIpc.onReopenable((r) => (this.reopenable = r));
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

  /** U10 Resolves with the tab it brought back, if any. */
  async reopen(): Promise<string | null> {
    try {
      const applied = await tabsIpc.reopen();
      settings.v.tabs = applied.tabs;
      return applied.made;
    } catch (e) {
      toast.error(String(e));
      return null;
    }
  }

  /** 1.51 Opens the tab in its own window; the main window shows where it went. */
  popOut(tab: Tab): void {
    void popOut(tabRoute(tab.id), label(tab));
  }

  /** 3.14 Closes the tab's own window; it's back in the main window. */
  dock(id: string): void {
    windowsIpc.closePanel(tabRoute(id)).catch((e) => toast.error(String(e)));
  }

  /** 3.14 The chat's window closes; it's back next to the stream. */
  dockChat(tileId: string): void {
    windowsIpc.closePanel(chatRoute(tileId)).catch((e) => toast.error(String(e)));
  }

  private setLive(list: ChannelInfo[]): void {
    this.live = Object.fromEntries(list.map((c) => [c.id, c]));
  }

  private setUnread(rooms: RoomUnread[], all: boolean): void {
    const next = all ? {} : { ...this.unread };
    for (const r of rooms) {
      const key = `${r.account ?? ''}/${r.login}`;
      if (r.level === 'seen') delete next[key];
      else next[key] = r.level;
    }
    this.unread = next;
  }

  private setPopped(routes: string[]): void {
    const ids = (prefix: string) => new Set(routes.filter((r) => r.startsWith(prefix)).map((r) => r.slice(prefix.length)));
    this.popped = ids(TAB_ROUTE);
    this.chats = ids(CHAT_ROUTE);
  }
}

export const tabs = new TabsStore();

/** Rust reads a removed account's tiles anonymously. */
function roomKey(account: string | null, login: string): string {
  const known = account && accounts.list.some((a) => a.userId === account);
  return `${known ? account : ''}/${login}`;
}

export function tiles(tab: Tab): Tile[] {
  return tab.columns.flatMap((c) => c.tiles);
}

/** 1.22 The alias, or the channel names. */
export function label(tab: Tab): string {
  return tab.alias ?? tiles(tab).map((t) => t.feed ?? t.channel.name).join(', ');
}

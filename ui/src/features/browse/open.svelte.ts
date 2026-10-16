// Opening channels from home and discover (1.6): their tab if one shows only them, else a
// new tab. H3 picks several and opens them side by side. H2 favorites live in settings.

import type { TabChannel } from '../../ipc/bindings/TabChannel';
import type { TileShow } from '../../ipc/bindings/TileShow';
import { settings } from '../../lib/settings.svelte';
import { toast } from '../../lib/toast.svelte';
import { tabs, tiles } from '../tabs/tabs.svelte';

/** 1.6 The three quick options. */
export type Layout = 'stream' | 'both' | 'chat';

export const LAYOUTS: Record<Layout, TileShow> = {
  stream: { player: true, chat: false },
  both: { player: true, chat: true },
  chat: { player: false, chat: true },
};

export const LAYOUT_LABELS: Record<Layout, string> = {
  stream: 'Stream',
  both: 'Stream and chat',
  chat: 'Chat',
};

/** H3 Same cap as Rust's `MAX_MULTI`: a 3 by 3 grid. */
export const MAX_PICKED = 9;

export interface Channel {
  id: string;
  login: string;
  name: string;
}

export interface OpenHow {
  layout?: Layout;
  /** Middle click: open the tab without leaving this view. */
  background?: boolean;
}

/** What home and discover need from the main window. */
export interface Nav {
  /** Shows a tab. */
  show: (tabId: string) => void;
  /** The account new tabs use. */
  account: () => string | null;
}

const plain = (c: Channel): TabChannel => ({ id: c.id, login: c.login, name: c.name });

export async function openChannel(nav: Nav, c: Channel, how: OpenHow = {}): Promise<void> {
  const own = tabs.list.find((t) => {
    const list = tiles(t);
    return list.length === 1 && list[0].channel.id === c.id;
  });
  const show = how.layout ? LAYOUTS[how.layout] : null;
  if (own) {
    const tile = tiles(own)[0];
    if (show && (tile.show.player !== show.player || tile.show.chat !== show.chat)) {
      await tabs.apply({ op: 'edit', tile: tile.id, channel: tile.channel, show, account: tile.account });
    }
    if (how.background) toast.info(`${c.name} is already open.`);
    else nav.show(own.id);
    return;
  }
  const id = await tabs.apply({
    op: 'add',
    channel: plain(c),
    show: show ?? tabs.lastShow,
    account: nav.account(),
    alias: null,
    into: null,
  });
  if (!id) return;
  if (how.background) {
    tabs.activate(null);
    toast.success(`Opened ${c.name} in a new tab.`);
  } else nav.show(id);
}

/** H3 Channels picked with Ctrl+click, across home and discover. */
class Picked {
  list = $state<Channel[]>([]);

  has(id: string): boolean {
    return this.list.some((c) => c.id === id);
  }

  toggle(c: Channel): void {
    if (this.has(c.id)) this.list = this.list.filter((x) => x.id !== c.id);
    else if (this.list.length >= MAX_PICKED) toast.info(`${MAX_PICKED} at most, side by side.`);
    else this.list = [...this.list, plain(c)];
  }

  clear(): void {
    this.list = [];
  }

  async open(nav: Nav, layout: Layout): Promise<void> {
    const channels = this.list.map(plain);
    if (!channels.length) return;
    const id = await tabs.apply({ op: 'addMany', channels, show: LAYOUTS[layout], account: nav.account() });
    if (!id) return;
    this.clear();
    nav.show(id);
  }
}

export const picked = new Picked();

/** H2 */
export function isFavorite(id: string): boolean {
  return settings.v.browse.favorites.some((c) => c.id === id);
}

export function toggleFavorite(c: Channel): void {
  const list = settings.v.browse.favorites;
  const favorites = isFavorite(c.id) ? list.filter((x) => x.id !== c.id) : [...list, plain(c)];
  void settings.patch('browse', { favorites });
}

// V110-U66 The ⋯ menu in a chat's header. In a tiled tab the tile's rows join it, in the
// Layout group (the same rows as the tile's own ⋯ menu).

import type { TokenKind } from '../../../ipc/bindings/TokenKind';
import type { MenuEntry } from '../../menuItems.ts';
import { group, item, type MenuDef, SEP } from '../types.ts';
import { lift, noop, row, slot, type Who } from './make.ts';
import { TILE_ITEMS, type Act, type TileCtx, tileSample } from './tile.ts';

export interface MoreCtx {
  /** A narrow chat: the viewer list is a switch here instead of a column. */
  tight: boolean;
  viewers: boolean;
  recording: boolean;
  searchHint?: string;
  /** An account is in this chat: Subscribe shows (greyed out with `lock` until the account can). */
  subscribe: null | { lock: TokenKind | null; unlock(): void; run(): void };
  overlay: Act | null;
  translating: boolean;
  muted: boolean;
  /** The Filters submenu; `null` where the chat takes no filters. */
  filters: MenuEntry | null;
  /** Pop-out chats and chats with no stream can't pop out. */
  popOut: Act | null;
  entries: MenuEntry[];
  plugins: MenuEntry[];
  /** The tile's rows; `null` where the chat is not in a tab. */
  tile: TileCtx | null;
  act: { viewers(): void; search(): void; record(): void; translate(): void; mute(): void; copyLink(): void; logs(): void };
}

const tile = (id: string) => {
  const def = TILE_ITEMS.find((i) => i.id === id)!;
  return lift<MoreCtx, TileCtx>(def, (c) => c.tile, `tile.${id}`);
};

export const MORE_DEF: MenuDef<MoreCtx> = {
  id: 'chat.more',
  label: 'Chat ⋯ menu',
  where: 'The ⋯ button in a chat’s header',
  items: [
    row('viewers', 'Viewers', 'users', (c) => (c.tight ? { label: 'Viewers', icon: 'users', checked: c.viewers, run: c.act.viewers } : null), {
      when: 'narrow chats',
    }),
    row('search', 'Search', 'search', (c) => ({ label: 'Search', icon: 'search', hint: c.searchHint, run: c.act.search })),
    row('record', 'Record chat', 'record', (c) => ({ label: c.recording ? 'Stop recording' : 'Record chat', icon: 'record', run: c.act.record }), {
      renamable: false,
    }),
    row(
      'subscribe',
      'Subscribe',
      'gift',
      (c) => (c.subscribe ? { label: 'Subscribe', icon: 'gift', lock: c.subscribe.lock, unlock: c.subscribe.unlock, run: c.subscribe.run } : null),
      { when: 'signed in' },
    ),
    row('overlay', 'Overlay', 'layers', (c) => (c.overlay ? { label: 'Overlay', icon: 'layers', note: 'This chat over games, always on top', run: c.overlay.run } : null)),
    row('translate', 'Translate messages', 'languages', (c) => ({ label: 'Translate messages', icon: 'languages', checked: c.translating, run: c.act.translate })),
    row('mute', 'Mute alerts', 'bellOff', (c) => ({ label: 'Mute alerts', icon: 'bellOff', checked: c.muted, run: c.act.mute })),
    row('filters', 'Filters', 'filter', (c) => c.filters),
    tile('theatre'),
    tile('sound'),
    tile('edit'),
    tile('hideChat'),
    tile('hideStream'),
    tile('detach'),
    tile('panels'),
    tile('reset'),
    tile('look'),
    tile('close'),
    // Palette only (3e).
    row('popOut', 'Pop out chat', 'popout', (c) => (c.popOut ? { label: 'Pop out chat', icon: 'popout', run: c.popOut.run } : null)),
    row('copyLink', 'Copy channel link', 'link', (c) => ({ label: 'Copy channel link', icon: 'link', run: c.act.copyLink })),
    row('logs', 'Open channel logs', 'scroll', (c) => ({ label: 'Open channel logs', icon: 'scroll', run: c.act.logs })),
    slot('entries', 'Your entries', (c) => c.entries),
    slot('plugins', 'Plugin entries', (c) => c.plugins),
  ],
  defaults: [
    item('search'),
    item('viewers'),
    SEP,
    item('translate'),
    item('mute'),
    item('filters'),
    SEP,
    item('record'),
    item('overlay'),
    SEP,
    item('tile.theatre'),
    item('tile.sound'),
    item('tile.edit'),
    item('tile.detach'),
    group('view', 'View', [item('tile.hideChat'), item('tile.hideStream'), item('tile.panels'), item('tile.reset'), item('tile.look')], 'layout'),
    SEP,
    item('subscribe'),
    item('entries'),
    item('plugins'),
    item('tile.close'),
  ],
};

export function moreSample(who: Who): MoreCtx {
  const a = who !== 'out';
  return {
    tight: true,
    viewers: false,
    recording: false,
    searchHint: 'Ctrl+F',
    subscribe: a ? { lock: null, unlock: noop, run: noop } : null,
    overlay: { run: noop },
    translating: false,
    muted: false,
    filters: { label: 'Filters', icon: 'filter', items: [{ label: 'Make filters in Settings → Filters', disabled: true }] },
    popOut: { run: noop },
    entries: [],
    plugins: [],
    tile: tileSample(who),
    act: { viewers: noop, search: noop, record: noop, translate: noop, mute: noop, copyLink: noop, logs: noop },
  };
}

// V110-U66 Right-click a channel in home or discover.

import { group, item, type MenuDef, SEP } from '../types.ts';
import { noop, row, type Who } from './make.ts';

export interface CardCtx {
  favorite: boolean;
  picked: boolean;
  act: {
    open(): void;
    background(): void;
    stream(): void;
    both(): void;
    chat(): void;
    favorite(): void;
    pick(): void;
    copyLink(): void;
  };
}

export const CARD_DEF: MenuDef<CardCtx> = {
  id: 'browse.card',
  label: 'Channel cards',
  where: 'Right-click a channel in Home or Discover',
  items: [
    row('open', 'Open', 'plus', (c) => ({ label: 'Open', icon: 'plus', run: c.act.open })),
    row('background', 'Open in the background', 'layers', (c) => ({ label: 'Open in the background', icon: 'layers', hint: 'Middle click', run: c.act.background })),
    row('stream', 'Open as stream', 'tv', (c) => ({ label: 'Stream', icon: 'tv', run: c.act.stream }), { renamable: false }),
    row('both', 'Open as stream and chat', 'panelRight', (c) => ({ label: 'Stream and chat', icon: 'panelRight', run: c.act.both }), { renamable: false }),
    row('chat', 'Open as chat', 'chat', (c) => ({ label: 'Chat', icon: 'chat', run: c.act.chat }), { renamable: false }),
    row(
      'favorite',
      'Favorite',
      'star',
      (c) =>
        c.favorite
          ? { label: 'Unfavorite', icon: 'star', run: c.act.favorite }
          : { label: 'Favorite', icon: 'star', note: 'Keeps it on top of home', run: c.act.favorite },
      { renamable: false },
    ),
    row('pick', 'Pick to watch side by side', 'columns', (c) => ({ label: c.picked ? 'Unpick' : 'Pick to watch side by side', icon: 'columns', hint: 'Ctrl+click', run: c.act.pick }), {
      renamable: false,
    }),
    // Palette only (3e).
    row('copyLink', 'Copy channel link', 'link', (c) => ({ label: 'Copy channel link', icon: 'link', run: c.act.copyLink })),
  ],
  defaults: [
    item('open'),
    item('background'),
    group('as', 'Open as', [item('stream'), item('both'), item('chat')], 'panelRight'),
    SEP,
    item('favorite'),
    item('pick'),
  ],
};

export function cardSample(_who: Who): CardCtx {
  return {
    favorite: false,
    picked: false,
    act: { open: noop, background: noop, stream: noop, both: noop, chat: noop, favorite: noop, pick: noop, copyLink: noop },
  };
}

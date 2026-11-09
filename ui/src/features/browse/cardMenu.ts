// Right-click on a channel in home or discover.

import type { MenuEntry } from '../../components/menu.svelte';
import { CARD_DEF } from '../../components/menus/defs/card';
import { menuEntries } from '../../components/menus/use';
import { copy } from '../../lib/clipboard';
import { type Channel, type Layout, type Nav, isFavorite, openChannel, picked, toggleFavorite } from './open.svelte';

export function cardMenu(nav: Nav, c: Channel): MenuEntry[] {
  const as = (layout: Layout) => () => void openChannel(nav, c, { layout });
  return menuEntries(
    CARD_DEF,
    {
      favorite: isFavorite(c.id),
      picked: picked.has(c.id),
      act: {
        open: () => void openChannel(nav, c),
        background: () => void openChannel(nav, c, { background: true }),
        stream: as('stream'),
        both: as('both'),
        chat: as('chat'),
        favorite: () => toggleFavorite(c),
        pick: () => picked.toggle(c),
        copyLink: () => void copy(`https://www.twitch.tv/${c.login}`, 'Link'),
      },
    },
    { values: { chan: c.login } },
  );
}

/** Click opens, Ctrl+click picks (H3), middle click opens in the background. */
export function cardClick(nav: Nav, c: Channel) {
  return {
    onclick: (e: MouseEvent) => {
      if (e.ctrlKey || e.metaKey) picked.toggle(c);
      else void openChannel(nav, c);
    },
    onauxclick: (e: MouseEvent) => {
      if (e.button === 1) void openChannel(nav, c, { background: true });
    },
    onmousedown: (e: MouseEvent) => {
      // Keeps Windows' autoscroll from starting on middle click.
      if (e.button === 1) e.preventDefault();
    },
  };
}

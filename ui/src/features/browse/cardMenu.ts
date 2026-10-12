// Right-click on a channel in home or discover.

import type { MenuEntry } from '../../components/menu.svelte';
import { type Channel, LAYOUT_LABELS, type Layout, type Nav, isFavorite, openChannel, picked, toggleFavorite } from './open.svelte';

const ICONS = { stream: 'tv', both: 'panelRight', chat: 'chat' } as const;

export function cardMenu(nav: Nav, c: Channel): MenuEntry[] {
  const layouts = (Object.keys(LAYOUT_LABELS) as Layout[]).map((layout) => ({
    label: LAYOUT_LABELS[layout],
    icon: ICONS[layout],
    run: () => void openChannel(nav, c, { layout }),
  }));
  return [
    { label: 'Open', icon: 'plus', run: () => void openChannel(nav, c) },
    { label: 'Open in the background', icon: 'layers', hint: 'Middle click', run: () => void openChannel(nav, c, { background: true }) },
    'separator',
    ...layouts,
    'separator',
    isFavorite(c.id)
      ? { label: 'Unstar', icon: 'star', run: () => toggleFavorite(c) }
      : { label: 'Star', icon: 'star', note: 'Keeps it on top of home', run: () => toggleFavorite(c) },
    {
      label: picked.has(c.id) ? 'Unpick' : 'Pick to watch side by side',
      icon: 'columns',
      hint: 'Ctrl+click',
      run: () => picked.toggle(c),
    },
  ];
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

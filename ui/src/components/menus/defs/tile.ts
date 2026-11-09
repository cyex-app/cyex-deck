// V110-U66 The ⋯ menu of a tile in a tiled tab, and the pieces tab menus and a chat's ⋯ share:
// hide the chat or the stream, move the panels, reset the layout, chat look, on/off switches.

import type { MenuEntry } from '../../menuItems.ts';
import { group, item, type ItemDef, type MenuDef, SEP } from '../types.ts';
import { noop, row, type Who } from './make.ts';
import type { Act, LayoutPart, ShowRow, Switch } from './parts.ts';
import { sampleLayout } from './parts.ts';

export type { Act, LayoutPart, ShowRow, Switch };

export interface TileCtx {
  /** The tile's own rows; `null` in a tab that is not tiled (a chat's ⋯ there only has the layout). */
  tile: null | {
    theatre: (Act & { hint?: string }) | null;
    sound: Switch | null;
    edit: Act | null;
    hideChat: ShowRow | null;
    hideStream: ShowRow | null;
    detach: Act;
    close: Act;
  };
  layout: LayoutPart | null;
  /** V108-N19 Text and emote size for this tab's chats (a submenu); `null` for a feed. */
  look: MenuEntry | null;
}

const t = <K extends keyof NonNullable<TileCtx['tile']>>(k: K) => (c: TileCtx) => c.tile?.[k] ?? null;

/** The tile's items; the chat's ⋯ menu reuses them with a `tile.` prefix. */
export const TILE_ITEMS: ItemDef<TileCtx>[] = [
  row('theatre', 'Theatre', 'expand', (c) => {
    const a = t('theatre')(c);
    return a ? { label: 'Theatre', icon: 'expand', hint: a.hint, run: a.run } : null;
  }),
  row('sound', 'Sound here', 'volume', (c) => {
    const a = t('sound')(c);
    return a ? { label: 'Sound here', icon: 'volume', checked: a.on, run: a.run } : null;
  }, { when: 'tabs with 2 streams' }),
  row('edit', 'Edit', 'pencil', (c) => {
    const a = t('edit')(c);
    return a ? { label: 'Edit', icon: 'pencil', run: a.run } : null;
  }),
  row('hideChat', 'Hide chat', 'chat', (c) => {
    const a = t('hideChat')(c);
    return a ? { label: a.label, icon: 'chat', run: a.run } : null;
  }, { renamable: false }),
  row('hideStream', 'Hide stream', 'tv', (c) => {
    const a = t('hideStream')(c);
    return a ? { label: a.label, icon: 'tv', run: a.run } : null;
  }, { renamable: false }),
  row('detach', 'Move to its own tab', 'popout', (c) => {
    const a = t('detach')(c);
    return a ? { label: 'Move to its own tab', icon: 'popout', run: a.run } : null;
  }),
  row('panels', 'Move panels', 'layout', (c) => (c.layout ? { label: 'Move panels', icon: 'layout', checked: c.layout.panels.on, run: c.layout.panels.run } : null), {
    when: 'tabs with 2 panels or more',
  }),
  row('reset', 'Reset layout', 'undo', (c) => (c.layout?.reset ? { label: 'Reset layout', icon: 'undo', run: c.layout.reset } : null), {
    when: 'after moving panels',
  }),
  row('look', 'Chat look', 'chat', (c) => c.look),
  row('close', 'Close', 'x', (c) => {
    const a = t('close')(c);
    return a ? { label: 'Close', icon: 'x', run: a.run } : null;
  }),
];

export const TILE_DEF: MenuDef<TileCtx> = {
  id: 'tabs.tile',
  label: 'Tiles',
  where: 'The ⋯ button on a tile in a tab with several channels',
  items: TILE_ITEMS,
  defaults: [
    item('theatre'),
    item('sound'),
    item('edit'),
    item('hideChat'),
    item('hideStream'),
    SEP,
    group('layout', 'Layout', [item('detach'), item('panels'), item('reset'), item('look')], 'layout'),
    SEP,
    item('close'),
  ],
};

const sampleLook: MenuEntry = { label: 'Chat look', icon: 'chat', items: [{ label: 'Text size', icon: 'wholeWord', run: noop }] };

export function tileSample(who: Who): TileCtx {
  const a = who !== 'out';
  return {
    tile: {
      theatre: { hint: 'T', run: noop },
      sound: a ? { on: true, run: noop } : null,
      edit: { run: noop },
      hideChat: { label: 'Hide chat', run: noop },
      hideStream: { label: 'Hide stream', run: noop },
      detach: { run: noop },
      close: { run: noop },
    },
    layout: sampleLayout(a),
    look: sampleLook,
  };
}

// V110-U66 Right-click a tab. With several tabs selected it acts on all of them, and the
// labels that count say how many.

import type { MenuEntry } from '../../menuItems.ts';
import { group, item, type ItemDef, type MenuDef, SEP } from '../types.ts';
import { noop, row, type Who } from './make.ts';
import { type Act, type LayoutPart, sampleLayout, type ShowRow, type Switch } from './parts.ts';

export interface TabCtx {
  /** Edit (a channel tab) or Rename (a feed tab); `null` for several tabs or a tab with several tiles (feed). */
  edit: (Act & { label: string; hint?: string }) | null;
  add: Act | null;
  /** Pop out tab / Bring back. */
  popOut: (Act & { popped: boolean }) | null;
  /** The tab plays out of sight: stop it. */
  held: Act | null;
  /** Mute stream / Unmute stream. */
  muteStream: (Act & { muted: boolean }) | null;
  /** Mute alerts / Unmute alerts for the tabs' channels (a ready entry). */
  alerts: MenuEntry | null;
  hideChat: ShowRow | null;
  hideStream: ShowRow | null;
  layout: LayoutPart | null;
  look: MenuEntry | null;
  mineFirst: MenuEntry | null;
  /** The Group submenu, labelled for one tab or several. */
  group: MenuEntry;
  onlyLive: Switch;
  lock: Switch;
  /** "Close" or "Close 3 tabs". */
  closeLabel: string;
  close: Act;
  /** How many other tabs there are, and how many come after this one: nothing to close greys the row out. */
  others: number;
  after: number;
  closeOthers: Act;
  closeAfter: Act;
  reopen: { can: boolean; hint?: string; run(): void };
  /** Palette only. */
  copyLink: Act | null;
  theatre: (Act & { hint?: string }) | null;
}

export const switchRow = (id: string, name: string, icon: 'filter' | 'lock', pick: (c: TabCtx) => Switch): ItemDef<TabCtx> =>
  row(id, name, icon, (c) => {
    const s = pick(c);
    return { label: name, icon, hint: s.hint, checked: s.on, run: s.run };
  });

export const TAB_DEF: MenuDef<TabCtx> = {
  id: 'tabs.tab',
  label: 'Tabs',
  where: 'Right-click a tab',
  items: [
    row('edit', 'Edit', 'pencil', (c) => (c.edit ? { label: c.edit.label, icon: 'pencil', hint: c.edit.hint, run: c.edit.run } : null), { renamable: false }),
    row('add', 'Add a channel here', 'columns', (c) => (c.add ? { label: 'Add a channel here', icon: 'columns', run: c.add.run } : null)),
    row('popOut', 'Pop out tab', 'popout', (c) => (c.popOut ? { label: c.popOut.popped ? 'Bring back' : 'Pop out tab', icon: c.popOut.popped ? 'undo' : 'popout', run: c.popOut.run } : null), {
      renamable: false,
    }),
    row('held', 'Stop playing', 'pause', (c) => (c.held ? { label: 'Stop playing', icon: 'pause', run: c.held.run } : null), { when: 'a stream playing out of sight' }),
    row(
      'muteStream',
      'Mute stream',
      'volumeOff',
      (c) =>
        c.muteStream
          ? { label: c.muteStream.muted ? 'Unmute stream' : 'Mute stream', icon: c.muteStream.muted ? 'volume' : 'volumeOff', run: c.muteStream.run }
          : null,
      { renamable: false },
    ),
    row('alerts', 'Mute alerts', 'bellOff', (c) => c.alerts, { renamable: false }),
    row('hideChat', 'Hide chat', 'chat', (c) => (c.hideChat ? { label: c.hideChat.label, icon: 'chat', run: c.hideChat.run } : null), { renamable: false }),
    row('hideStream', 'Hide stream', 'tv', (c) => (c.hideStream ? { label: c.hideStream.label, icon: 'tv', run: c.hideStream.run } : null), { renamable: false }),
    row('panels', 'Move panels', 'layout', (c) => (c.layout ? { label: 'Move panels', icon: 'layout', checked: c.layout.panels.on, run: c.layout.panels.run } : null)),
    row('reset', 'Reset layout', 'undo', (c) => (c.layout?.reset ? { label: 'Reset layout', icon: 'undo', run: c.layout.reset } : null), { when: 'after moving panels' }),
    row('look', 'Chat look', 'chat', (c) => c.look),
    row('mineFirst', 'Mine first', 'pickaxe', (c) => c.mineFirst, { when: 'miner accounts' }),
    row('group', 'Group', 'tag', (c) => c.group, { renamable: false }),
    switchRow('onlyLive', 'Only live tabs', 'filter', (c) => c.onlyLive),
    switchRow('lock', 'Lock tabs', 'lock', (c) => c.lock),
    row('close', 'Close', 'x', (c) => ({ label: c.closeLabel, icon: 'x', run: c.close.run }), { renamable: false }),
    row('closeOthers', 'Close other tabs', 'x', (c) => ({ label: 'Close other tabs', icon: 'x', disabled: !c.others, run: c.closeOthers.run })),
    row('closeAfter', 'Close tabs after this', 'x', (c) => ({ label: 'Close tabs after this', icon: 'x', disabled: !c.after, run: c.closeAfter.run })),
    row('reopen', 'Reopen closed tab', 'undo', (c) => ({
      label: 'Reopen closed tab',
      icon: 'undo',
      hint: c.reopen.hint,
      disabled: !c.reopen.can,
      run: c.reopen.run,
    })),
    // Palette only (3e).
    row('copyLink', 'Copy channel link', 'link', (c) => (c.copyLink ? { label: 'Copy channel link', icon: 'link', run: c.copyLink.run } : null)),
    row('theatre', 'Theatre', 'expand', (c) => (c.theatre ? { label: 'Theatre', icon: 'expand', hint: c.theatre.hint, run: c.theatre.run } : null)),
  ],
  defaults: [
    item('edit'),
    item('add'),
    item('popOut'),
    item('muteStream'),
    item('held'),
    item('alerts'),
    item('mineFirst'),
    SEP,
    group('view', 'View', [item('hideChat'), item('hideStream'), item('panels'), item('reset'), item('look')], 'layout'),
    item('group'),
    SEP,
    item('close'),
    item('closeOthers'),
    item('closeAfter'),
    item('reopen'),
  ],
};

export function tabSample(who: Who): TabCtx {
  const a = who !== 'out';
  const act = { run: noop };
  return {
    edit: { label: 'Edit', hint: 'Double-click', run: noop },
    add: act,
    popOut: { popped: false, run: noop },
    held: a ? act : null,
    muteStream: { muted: false, run: noop },
    alerts: { label: 'Mute alerts', icon: 'bellOff', run: noop },
    hideChat: { label: 'Hide chat', run: noop },
    hideStream: { label: 'Hide stream', run: noop },
    layout: sampleLayout(),
    look: { label: 'Chat look', icon: 'chat', items: [{ label: 'Text size', run: noop }] },
    mineFirst: a ? { label: 'Mine first', icon: 'pickaxe', checked: false, run: noop } : null,
    group: { label: 'Group', icon: 'tag', items: [{ label: 'New group', icon: 'plus', run: noop }, { label: 'Blue', swatch: '#145aff', run: noop }] },
    onlyLive: { on: false, run: noop },
    lock: { on: false, run: noop },
    closeLabel: 'Close',
    close: act,
    others: 2,
    after: 1,
    closeOthers: act,
    closeAfter: act,
    reopen: { can: a, run: noop },
    copyLink: act,
    theatre: act,
  };
}

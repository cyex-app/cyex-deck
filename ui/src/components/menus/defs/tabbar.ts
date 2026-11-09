// V110-U66 Right-click the tab bar between tabs, and right-click a tab group's header.
// Built-in layouts unchanged.

import type { MenuEntry } from '../../menuItems.ts';
import { item, type MenuDef, SEP } from '../types.ts';
import { noop, row, type Who } from './make.ts';
import type { Act, Switch } from './parts.ts';

export interface BarCtx {
  add: Act & { hint?: string };
  onlyLive: Switch;
  lock: Switch;
  /** The "Tab bar position" submenu. */
  position: MenuEntry;
  reopen: { can: boolean; hint?: string; run(): void };
}

export const BAR_DEF: MenuDef<BarCtx> = {
  id: 'tabs.bar',
  label: 'Tab bar',
  where: 'Right-click the tab bar between tabs',
  items: [
    row('add', 'Add a tab', 'plus', (c) => ({ label: 'Add a tab', icon: 'plus', hint: c.add.hint, run: c.add.run })),
    row('onlyLive', 'Only live tabs', 'filter', (c) => ({ label: 'Only live tabs', icon: 'filter', hint: c.onlyLive.hint, checked: c.onlyLive.on, run: c.onlyLive.run })),
    row('lock', 'Lock tabs', 'lock', (c) => ({ label: 'Lock tabs', icon: 'lock', hint: c.lock.hint, checked: c.lock.on, run: c.lock.run })),
    row('position', 'Tab bar position', 'layout', (c) => c.position),
    row('reopen', 'Reopen closed tab', 'undo', (c) => ({ label: 'Reopen closed tab', icon: 'undo', hint: c.reopen.hint, disabled: !c.reopen.can, run: c.reopen.run })),
  ],
  defaults: [item('add'), item('onlyLive'), item('lock'), item('position'), SEP, item('reopen')],
};

export function barSample(who: Who): BarCtx {
  return {
    add: { hint: 'Ctrl+T', run: noop },
    onlyLive: { on: false, run: noop },
    lock: { on: false, run: noop },
    position: { label: 'Tab bar position', icon: 'layout', items: [{ label: 'Top', checked: true, run: noop }] },
    reopen: { can: who !== 'out', run: noop },
  };
}

export interface GroupCtx {
  rename: Act;
  /** The "Color" submenu (swatches). */
  color: MenuEntry;
  ungroup: Act;
  /** "Close 3 tabs". */
  closeLabel: string;
  close: Act;
}

export const GROUP_DEF: MenuDef<GroupCtx> = {
  id: 'tabs.group',
  label: 'Tab groups',
  where: 'Right-click a tab group’s header',
  items: [
    row('rename', 'Rename', 'pencil', (c) => ({ label: 'Rename', icon: 'pencil', run: c.rename.run })),
    row('color', 'Color', 'palette', (c) => c.color, { renamable: false }),
    row('ungroup', 'Ungroup', 'tag', (c) => ({ label: 'Ungroup', icon: 'tag', run: c.ungroup.run })),
    row('close', 'Close tabs', 'x', (c) => ({ label: c.closeLabel, icon: 'x', run: c.close.run }), { renamable: false }),
  ],
  defaults: [item('rename'), item('color'), item('ungroup'), SEP, item('close')],
};

export function groupSample(_who: Who): GroupCtx {
  return {
    rename: { run: noop },
    color: { label: 'Color', swatch: '#145aff', items: [{ label: 'Blue', swatch: '#145aff', checked: true, run: noop }] },
    ungroup: { run: noop },
    closeLabel: 'Close 3 tabs',
    close: { run: noop },
  };
}

// Right-click menus for tabs and tab groups (1.18, 1.51, U3, U5).

import type { MenuEntry } from '../../components/menu.svelte';
import type { Tab } from '../../ipc/bindings/Tab';
import type { TabGroup } from '../../ipc/bindings/TabGroup';
import { bindingOf } from '../../lib/hotkeys.svelte';
import { settings } from '../../lib/settings.svelte';
import { openAdd, openEdit, openGroupDialog } from './dialog.svelte';
import { COLOR_ORDER, GROUP_COLORS } from './groupColors';
import { tabs, tiles } from './tabs.svelte';

/** Menu for `tab`. `targets` are the selected tabs it acts on (`tab` included). */
export function tabMenu(tab: Tab, targets: Tab[]): MenuEntry[] {
  const ids = targets.map((t) => t.id);
  const one = targets.length === 1;
  const all = tabs.list;
  const index = all.findIndex((t) => t.id === tab.id);
  const after = all.slice(index + 1).map((t) => t.id);
  const others = all.filter((t) => !ids.includes(t.id)).map((t) => t.id);
  const popped = tabs.popped.has(tab.id);
  const feed = tiles(tab).some((t) => t.feed);

  const entries: MenuEntry[] = [];
  if (one && !feed) {
    entries.push(
      { label: 'Edit', icon: 'pencil', hint: 'Double-click', run: () => openEdit(tab.id, tiles(tab)[0].id) },
      { label: 'Add a channel here', icon: 'columns', run: () => openAdd(tab.id) },
      popped
        ? { label: 'Bring back', icon: 'undo', run: () => tabs.dock(tab.id) }
        : { label: 'Pop out', icon: 'popout', run: () => tabs.popOut(tab) },
    );
  }
  const mute = muteEntry(targets);
  if (mute) entries.push(mute);
  entries.push(
    { label: one ? 'Group' : `Group ${targets.length} tabs`, icon: 'tag', items: groupItems(targets) },
    'separator',
    { label: one ? 'Close' : `Close ${targets.length} tabs`, icon: 'x', run: () => tabs.close(ids) },
    { label: 'Close other tabs', disabled: !others.length, run: () => tabs.close(others) },
    { label: 'Close tabs after this', disabled: !after.length, run: () => tabs.close(after) },
    reopenEntry(),
  );
  return entries;
}

/** C27 Keeps the tabs' chats from playing sounds or flashing. */
function muteEntry(targets: Tab[]): MenuEntry | null {
  const logins = targets.flatMap(tiles).filter((t) => !t.feed && t.channel.login).map((t) => t.channel.login);
  if (!logins.length) return null;
  const muted = settings.v.highlights.muted;
  const all = logins.every((l) => muted.includes(l));
  const next = all ? muted.filter((l) => !logins.includes(l)) : [...new Set([...muted, ...logins])];
  return {
    label: 'Mute sounds',
    icon: 'bellOff',
    checked: all,
    run: () => void settings.patch('highlights', { muted: next }),
  };
}

/** U10 */
function reopenEntry(): MenuEntry {
  return {
    label: 'Reopen closed tab',
    icon: 'undo',
    hint: bindingOf('tabs.reopen') || undefined,
    disabled: !tabs.reopenable,
    run: () => void tabs.reopen(),
  };
}

function groupItems(targets: Tab[]): MenuEntry[] {
  const ids = targets.map((t) => t.id);
  const items: MenuEntry[] = [{ label: 'New group', icon: 'plus', run: () => openGroupDialog({ tabs: ids }) }];
  for (const g of tabs.groups) {
    const inIt = targets.every((t) => t.group === g.id);
    items.push({
      label: g.name || GROUP_COLORS[g.color].name,
      swatch: GROUP_COLORS[g.color].css,
      checked: inIt,
      run: () => void tabs.apply({ op: 'group', tabs: ids, group: g.id }),
    });
  }
  if (targets.some((t) => t.group)) {
    items.push('separator', {
      label: 'Remove from group',
      run: () => void tabs.apply({ op: 'group', tabs: ids, group: null }),
    });
  }
  return items;
}

export function groupMenu(group: TabGroup): MenuEntry[] {
  const members = tabs.list.filter((t) => t.group === group.id).map((t) => t.id);
  const edit = (patch: { name?: string; color?: TabGroup['color'] }) =>
    void tabs.apply({
      op: 'editGroup',
      group: group.id,
      name: patch.name ?? null,
      color: patch.color ?? null,
      collapsed: null,
    });
  return [
    { label: 'Rename', icon: 'pencil', run: () => openGroupDialog({ group: group.id }) },
    {
      label: 'Color',
      swatch: GROUP_COLORS[group.color].css,
      items: COLOR_ORDER.map((c) => ({
        label: GROUP_COLORS[c].name,
        swatch: GROUP_COLORS[c].css,
        checked: c === group.color,
        run: () => edit({ color: c }),
      })),
    },
    { label: 'Ungroup', icon: 'tag', run: () => void tabs.apply({ op: 'ungroup', group: group.id }) },
    'separator',
    { label: `Close ${members.length} tabs`, icon: 'x', danger: true, run: () => tabs.close(members) },
  ];
}

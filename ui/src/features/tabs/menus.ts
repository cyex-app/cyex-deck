// Right-click menus for tabs, tab groups and the bar itself (1.18, 1.51, U3, U5), with
// "Mine first" (1.44), hiding a tile's chat or stream (V101-20), moving its panels (V102-23,
// V103-05), stopping a stream that plays out of sight (V102-15), muting a tab (V106-25) and
// "Only live tabs" (F17).

import type { MenuEntry } from '../../components/menu.svelte';
import type { Tab } from '../../ipc/bindings/Tab';
import type { TabGroup } from '../../ipc/bindings/TabGroup';
import type { Tile } from '../../ipc/bindings/Tile';
import { bindingOf } from '../../lib/hotkeys.svelte';
import { settings } from '../../lib/settings.svelte';
import { mineFirstEntry } from '../miner/first';
import { modviewPlace } from '../modview/where.svelte';
import { tileState } from '../player/players.svelte';
import { openAdd, openEdit, openGroupDialog } from './dialog.svelte';
import { COLOR_ORDER, GROUP_COLORS } from './groupColors';
import { dragged, layout } from './layout.svelte';
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
        : { label: 'Pop out tab', icon: 'popout', run: () => tabs.popOut(tab) },
    );
    if (tiles(tab).length === 1) entries.push(...showEntries(tiles(tab)[0]));
  }
  if (one && feed && tiles(tab).length === 1) {
    entries.push({ label: 'Rename', icon: 'pencil', hint: 'Double-click', run: () => openEdit(tab.id, tiles(tab)[0].id) });
  }
  if (one && tabs.held === tab.id) {
    entries.push({ label: 'Stop playing', icon: 'pause', run: () => (tabs.held = null) });
  }
  // V106-25 The only place a whole tab is muted (keyboard: Menu key on a focused tab). A muted
  // tab can always be unmuted, popped out or with nothing playing.
  if (one && tabs.muted.has(tab.id)) {
    entries.push({ label: 'Unmute stream', icon: 'volume', run: () => void tabs.mute(tab.id, false) });
  } else if (one && tabs.canPlay(tab)) {
    entries.push({ label: 'Mute stream', icon: 'volumeOff', run: () => void tabs.mute(tab.id, true) });
  }
  const mute = muteEntry(targets);
  if (mute) entries.push(mute);
  // A popped-out tab's panels move in its own window, from the chat's menu.
  if (one && !popped) entries.push(...layoutEntries(tab));
  const first = one && !feed ? mineFirstEntry(tiles(tab).map((t) => t.channel)) : null;
  if (first) entries.push(first);
  entries.push(
    { label: one ? 'Group' : `Group ${targets.length} tabs`, icon: 'tag', items: groupItems(targets) },
    onlyLiveEntry(),
    'separator',
    { label: one ? 'Close' : `Close ${targets.length} tabs`, icon: 'x', run: () => tabs.close(ids) },
    { label: 'Close other tabs', icon: 'x', disabled: !others.length, run: () => tabs.close(others) },
    { label: 'Close tabs after this', icon: 'x', disabled: !after.length, run: () => tabs.close(after) },
    reopenEntry(),
  );
  return entries;
}

/** A tile's own entries in a tiled tab (U3): edit, hide (V101-20), split off or close. In a
 * chat they follow the chat's entries in one menu (V101-21). */
export function tileEntries(tab: Tab, tile: Tile): MenuEntry[] {
  return [
    ...(tile.feed ? [] : [{ label: 'Edit', icon: 'pencil' as const, run: () => openEdit(tab.id, tile.id) }, ...showEntries(tile)]),
    { label: 'Move to its own tab', icon: 'popout', run: () => void tabs.apply({ op: 'detach', tile: tile.id, before: null }) },
    ...layoutEntries(tab),
    'separator',
    { label: 'Close', icon: 'x', run: () => void tabs.apply({ op: 'closeTile', tile: tile.id }) },
  ];
}

/** V102-23 Unlock the tab's panels to drag them, or put every chat back. V103-05 Wherever
 * a tile has two panels or more (stream, chat, mod view, a clip beside the chat). */
export function layoutEntries(tab: Tab): MenuEntry[] {
  const panels = (t: Tile) =>
    Number(t.show.player || (t.show.chat && tileState(t.id).side !== null)) + Number(t.show.chat) + Number(modviewPlace.isIn(t.id));
  if (!tiles(tab).some((t) => panels(t) > 1) && !dragged(tab)) return [];
  const on = layout.isUnlocked(tab.id);
  return [
    ...(dragged(tab) ? [{ label: 'Reset layout', icon: 'undo' as const, run: () => layout.reset(tab) }] : []),
    { label: 'Move panels', icon: 'layout', checked: on, run: () => layout.setUnlocked(tab.id, !on) },
  ];
}

/** V101-20 Hide the chat or the stream, or bring it back, without the edit dialog. */
export function showEntries(tile: Tile): MenuEntry[] {
  const set = (player: boolean, chat: boolean) => () => void tabs.apply({ op: 'show', tile: tile.id, show: { player, chat } });
  const { player, chat } = tile.show;
  if (player && chat) {
    return [
      { label: 'Hide chat', icon: 'chat', run: set(true, false) },
      { label: 'Hide stream', icon: 'tv', run: set(false, true) },
    ];
  }
  return [player ? { label: 'Show chat', icon: 'chat', run: set(true, true) } : { label: 'Show stream', icon: 'tv', run: set(true, true) }];
}

/** C27 Keeps the tabs' chats from playing sounds or flashing. */
function muteEntry(targets: Tab[]): MenuEntry | null {
  const logins = targets.flatMap(tiles).filter((t) => !t.feed && t.channel.login).map((t) => t.channel.login);
  if (!logins.length) return null;
  const muted = settings.v.highlights.muted;
  const all = logins.every((l) => muted.includes(l));
  const next = all ? muted.filter((l) => !logins.includes(l)) : [...new Set([...muted, ...logins])];
  // V106 a-5 Says what it does next, like Mute stream.
  return {
    label: all ? 'Unmute alerts' : 'Mute alerts',
    icon: all ? 'bell' : 'bellOff',
    run: () => void settings.patch('highlights', { muted: next }),
  };
}

/** Right-click on the bar between tabs. */
export function barMenu(): MenuEntry[] {
  return [
    { label: 'Add a tab', icon: 'plus', hint: bindingOf('tabs.new') || undefined, run: () => openAdd() },
    onlyLiveEntry(),
    'separator',
    reopenEntry(),
  ];
}

/** F17 Hides tabs whose channels are offline. */
function onlyLiveEntry(): MenuEntry {
  const on = settings.v.appearance.tabOnlyLive;
  return {
    label: 'Only live tabs',
    icon: 'filter',
    hint: bindingOf('tabs.onlyLive') || undefined,
    checked: on,
    run: () => void settings.patch('appearance', { tabOnlyLive: !on }),
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
      icon: 'x',
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
    // V101-30 Not red: closed tabs come back with Reopen closed tab.
    { label: `Close ${members.length} tabs`, icon: 'x', run: () => tabs.close(members) },
  ];
}

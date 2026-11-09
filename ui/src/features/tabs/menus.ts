// Right-click menus for tabs, tab groups and the bar itself (1.18, 1.51, U3, U5), with
// "Mine first" (1.44), hiding a tile's chat or stream (V101-20), moving its panels (V102-23,
// V103-05), stopping a stream that plays out of sight (V102-15), muting a tab (V106-25) and
// "Only live tabs" (F17). V110-U66 Which rows show, and where, comes from the menu's layout
// (Settings › Menus); this file says what each row does.

import type { MenuEntry } from '../../components/menu.svelte';
import type { LayoutPart, ShowRow, Switch } from '../../components/menus/defs/parts';
import { TAB_DEF, type TabCtx } from '../../components/menus/defs/tab';
import { BAR_DEF, type BarCtx, closeTabs, GROUP_DEF, type GroupCtx } from '../../components/menus/defs/tabbar';
import { TILE_DEF, type TileCtx } from '../../components/menus/defs/tile';
import { menuEntries } from '../../components/menus/use';
import type { Tab } from '../../ipc/bindings/Tab';
import type { TabBarSide } from '../../ipc/bindings/TabBarSide';
import type { TabGroup } from '../../ipc/bindings/TabGroup';
import type { Tile } from '../../ipc/bindings/Tile';
import { copy } from '../../lib/clipboard';
import { bindingOf } from '../../lib/hotkeys.svelte';
import { settings } from '../../lib/settings.svelte';
import { mineFirstEntry } from '../miner/first';
import { modviewPlace } from '../modview/where.svelte';
import { tileState } from '../player/players.svelte';
import { openAdd, openEdit, openGroupDialog } from './dialog.svelte';
import { COLOR_ORDER, GROUP_COLORS } from './groupColors';
import { setBarSide } from './barSide';
import { audioFocus } from './audioFocus.svelte';
import { takeSound } from './takeSound';
import { dragged, layout } from './layout.svelte';
import { CHAT_SIZES, EMOTE_SIZES, isDefaultLook, withLook } from './look';
import { tabs, streamIds, tiles } from './tabs.svelte';

/** What a tab menu is opened with: what applies to these tabs, and what each row does.
 * `targets` are the selected tabs it acts on (`tab` included). */
export function tabCtx(tab: Tab, targets: Tab[]): TabCtx {
  const ids = targets.map((t) => t.id);
  const one = targets.length === 1;
  const all = tabs.list;
  const index = all.findIndex((t) => t.id === tab.id);
  const after = all.slice(index + 1).map((t) => t.id);
  const others = all.filter((t) => !ids.includes(t.id)).map((t) => t.id);
  const popped = tabs.popped.has(tab.id);
  const list = tiles(tab);
  const feed = list.some((t) => t.feed);
  const first = list[0];
  const login = first?.channel.login;
  const show = one && !feed && list.length === 1 ? showRows(first) : null;
  const muted = one && tabs.muted.has(tab.id);
  const edit = (label: string) => ({ label, hint: 'Double-click', run: () => openEdit(tab.id, first.id) });
  return {
    edit: one && !feed ? edit('Edit') : one && feed && list.length === 1 ? edit('Rename') : null,
    add: one && !feed ? { run: () => openAdd(tab.id) } : null,
    popOut: one && !feed ? { popped, run: () => (popped ? tabs.dock(tab.id) : tabs.popOut(tab)) } : null,
    held: one && tabs.held === tab.id ? { run: () => (tabs.held = null) } : null,
    // V106-25 The only place a whole tab is muted (keyboard: Menu key on a focused tab). A muted
    // tab can always be unmuted, popped out or with nothing playing.
    muteStream: muted || (one && tabs.canPlay(tab)) ? { muted, run: () => void tabs.mute(tab.id, !muted) } : null,
    alerts: muteEntry(targets),
    hideChat: show?.hideChat ?? null,
    hideStream: show?.hideStream ?? null,
    // A popped-out tab's panels move in its own window, from the chat's menu.
    layout: one && !popped ? layoutPart(tab) : null,
    look: one && !feed ? lookEntry(tab) : null,
    mineFirst: one && !feed ? mineFirstEntry(list.map((t) => t.channel)) : null,
    group: { label: one ? 'Group' : `Group ${targets.length} tabs`, icon: 'tag', items: groupItems(targets) },
    onlyLive: onlyLiveSwitch(),
    lock: lockSwitch(),
    closeLabel: one ? 'Close' : `Close ${targets.length} tabs`,
    close: { run: () => tabs.close(ids) },
    others: others.length,
    after: after.length,
    closeOthers: { run: () => tabs.close(others) },
    closeAfter: { run: () => tabs.close(after) },
    reopen: reopenPart(),
    copyLink: one && !feed && login ? { run: () => void copy(`https://www.twitch.tv/${login}`, 'Link') } : null,
    theatre:
      one && !feed && list.length === 1 && first.show.player
        ? { hint: bindingOf('view.theatre') || undefined, run: () => audioFocus.enterTheatre(tab.id, first.id) }
        : null,
  };
}

/** Menu for `tab`. `targets` are the selected tabs it acts on (`tab` included). */
export function tabMenu(tab: Tab, targets: Tab[]): MenuEntry[] {
  return menuEntries(TAB_DEF, tabCtx(tab, targets), { values: { chan: tiles(tab)[0]?.channel.login ?? '' } });
}

/** What a tile's menu is opened with (U3). Without a tile (`null`: a chat in a tab that is
 * not tiled) only the layout rows are there. */
export function tileCtx(tab: Tab, tile: Tile | null): TileCtx {
  if (!tile) return { tile: null, layout: layoutPart(tab), look: null };
  const streams = streamIds(tab);
  const show = tile.feed ? null : showRows(tile);
  return {
    tile: {
      theatre:
        tile.show.player && !tile.feed
          ? { hint: bindingOf('view.theatre') || undefined, run: () => audioFocus.enterTheatre(tab.id, tile.id) }
          : null,
      sound:
        tile.show.player && !tile.feed && streams.length > 1 && settings.v.appearance.audioFocus
          ? { on: !tabs.focusHushed(tab, tile.id), run: () => takeSound(tab, tile) }
          : null,
      edit: tile.feed ? null : { run: () => openEdit(tab.id, tile.id) },
      hideChat: show?.hideChat ?? null,
      hideStream: show?.hideStream ?? null,
      detach: { run: () => void tabs.apply({ op: 'detach', tile: tile.id, before: null }) },
      close: { run: () => void tabs.apply({ op: 'closeTile', tile: tile.id }) },
    },
    layout: layoutPart(tab),
    look: tile.feed ? null : lookEntry(tab),
  };
}

/** A tile's own entries in a tiled tab (U3): edit, hide (V101-20), split off or close. In a
 * chat they follow the chat's entries in one menu (V101-21). */
export function tileEntries(tab: Tab, tile: Tile): MenuEntry[] {
  return menuEntries(TILE_DEF, tileCtx(tab, tile), { values: { chan: tile.channel.login } });
}

/** V102-23 Unlock the tab's panels to drag them, or put every chat back. V103-05 Wherever
 * a tile has two panels or more (stream, chat, mod view, a clip beside the chat).
 * V107-R2 Or the tab has two tiles or more, e.g. chats only, to drag them around. `null`
 * where there is nothing to move. */
export function layoutPart(tab: Tab): LayoutPart | null {
  const panels = (t: Tile) =>
    Number(t.show.player || (t.show.chat && tileState(t.id).side !== null)) + Number(t.show.chat) + Number(modviewPlace.isIn(t.id));
  const all = tiles(tab);
  if (all.length < 2 && !all.some((t) => panels(t) > 1) && !dragged(tab)) return null;
  const on = layout.isUnlocked(tab.id);
  return {
    reset: dragged(tab) ? () => layout.reset(tab) : null,
    panels: { on, run: () => layout.setUnlocked(tab.id, !on) },
  };
}

/** V108-N19 Text and emote size for this tab's chats, over Settings. */
function lookEntry(tab: Tab): MenuEntry {
  const set = (patch: { chatSize?: number | null; emoteSize?: number | null }) => () =>
    void tabs.apply({ op: 'look', tab: tab.id, look: withLook(tab.look, patch) });
  const { chatSize, emoteSize } = tab.look ?? {};
  const pick = (value: number | null | undefined, sizes: readonly number[], unit: string, key: 'chatSize' | 'emoteSize'): MenuEntry[] => [
    { label: 'Default', checked: value == null, run: set({ [key]: null }) },
    ...sizes.map((n) => ({ label: `${n}${unit}`, checked: value === n, run: set({ [key]: n }) })),
  ];
  return {
    label: 'Chat look',
    icon: 'chat',
    items: [
      { label: 'Text size', icon: 'wholeWord', hint: chatSize != null ? `${chatSize}px` : undefined, items: pick(chatSize, CHAT_SIZES, 'px', 'chatSize') },
      { label: 'Emote size', icon: 'smile', hint: emoteSize != null ? `${emoteSize}%` : undefined, items: pick(emoteSize, EMOTE_SIZES, '%', 'emoteSize') },
      'separator',
      {
        label: 'Reset to default',
        icon: 'undo',
        disabled: isDefaultLook(tab.look),
        run: () => void tabs.apply({ op: 'look', tab: tab.id, look: { chatSize: null, emoteSize: null } }),
      },
    ],
  };
}

/** V101-20 Hide the chat or the stream, or bring it back, without the edit dialog. */
function showRows(tile: Tile): { hideChat: ShowRow | null; hideStream: ShowRow | null } {
  const set = (player: boolean, chat: boolean) => () => void tabs.apply({ op: 'show', tile: tile.id, show: { player, chat } });
  const { player, chat } = tile.show;
  if (player && chat) {
    return { hideChat: { label: 'Hide chat', run: set(true, false) }, hideStream: { label: 'Hide stream', run: set(false, true) } };
  }
  return player
    ? { hideChat: { label: 'Show chat', run: set(true, true) }, hideStream: null }
    : { hideChat: null, hideStream: { label: 'Show stream', run: set(true, true) } };
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
  const ctx: BarCtx = {
    add: { hint: bindingOf('tabs.new') || undefined, run: () => openAdd() },
    onlyLive: onlyLiveSwitch(),
    lock: lockSwitch(),
    position: positionEntry(),
    reopen: reopenPart(),
  };
  return menuEntries(BAR_DEF, ctx);
}

/** V110-U52 Where the bar sits, with "Above chat" (over the chat column, while the chat is beside the stream). */
function positionEntry(): MenuEntry {
  const now = settings.v.appearance.tabBarSide;
  const side = (label: string, value: TabBarSide, hint?: string): MenuEntry => ({
    label,
    hint,
    checked: now === value,
    run: () => setBarSide(value),
  });
  return {
    label: 'Tab bar position',
    icon: 'layout',
    items: [
      side('Top', 'top'),
      side('Above chat', 'aboveChat', 'Only when the chat is beside the stream.'),
      side('Bottom', 'bottom'),
      side('Left', 'left'),
      side('Right', 'right'),
    ],
  };
}

/** F17 Hides tabs whose channels are offline. */
function onlyLiveSwitch(): Switch {
  const on = settings.v.appearance.tabOnlyLive;
  return { on, hint: bindingOf('tabs.onlyLive') || undefined, run: () => void settings.patch('appearance', { tabOnlyLive: !on }) };
}

/** V109-U05 Stops tabs from being dragged or moved by key. */
function lockSwitch(): Switch {
  const on = settings.v.appearance.tabLock;
  return { on, hint: bindingOf('tabs.lock') || undefined, run: () => void settings.patch('appearance', { tabLock: !on }) };
}

/** U10 */
function reopenPart() {
  return { can: tabs.reopenable, hint: bindingOf('tabs.reopen') || undefined, run: () => void tabs.reopen() };
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
  const ctx: GroupCtx = {
    rename: { run: () => openGroupDialog({ group: group.id }) },
    color: {
      label: 'Color',
      swatch: GROUP_COLORS[group.color].css,
      items: COLOR_ORDER.map((c) => ({
        label: GROUP_COLORS[c].name,
        swatch: GROUP_COLORS[c].css,
        checked: c === group.color,
        run: () => edit({ color: c }),
      })),
    },
    ungroup: { run: () => void tabs.apply({ op: 'ungroup', group: group.id }) },
    // V101-30 Not red: closed tabs come back with Reopen closed tab.
    closeLabel: closeTabs(members.length),
    close: { run: () => tabs.close(members) },
  };
  return menuEntries(GROUP_DEF, ctx);
}

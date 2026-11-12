// V110-U66 Every menu the app lets people arrange: its def and a fake context for previews.
// Slice C's manager imports this. Pure.

import type { MenuDef, MenuId } from '../types.ts';
import { CARD_DEF, cardSample } from './card.ts';
import { CLIP_DEF, clipSample } from './clip.ts';
import { EMOTE_DEF, emoteSample } from './emote.ts';
import { FEED_MESSAGE_DEF, FEED_NAME_DEF, feedMessageSample, feedNameSample } from './feed.ts';
import type { Who } from './make.ts';
import { MESSAGE_DEF, messageSample } from './message.ts';
import { MORE_DEF, moreSample } from './more.ts';
import { NAME_DEF, nameSample } from './name.ts';
import { PLAYER_DEF, playerSample } from './player.ts';
import { TAB_DEF, tabSample } from './tab.ts';
import { BAR_DEF, barSample, GROUP_DEF, groupSample } from './tabbar.ts';
import { TILE_DEF, tileSample } from './tile.ts';
import { WHISPER_DEF, whisperSample } from './whispers.ts';

/** Who a preview pretends to be: someone watching, a mod of the channel, or nobody signed in. */
export type PreviewAs = Who;
export const PREVIEW_AS: { id: PreviewAs; label: string }[] = [
  { id: 'viewer', label: 'Viewer' },
  { id: 'mod', label: 'Mod' },
  { id: 'out', label: 'Signed out' },
];

// Each def has its own context type.
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type AnyDef = MenuDef<any>;

/** V110-N9 The manager's small headings over its list. */
export const MENU_GROUPS = ['Chat', 'Feeds', 'Tabs', 'Watching'] as const;
export type MenuGroup = (typeof MENU_GROUPS)[number];

export interface MenuInfo {
  def: AnyDef;
  /** V110-N9 The heading it sits under in the manager's list. */
  group: MenuGroup;
  /** V110-N9 Its name inside that heading (`def.label` is the full name: editor title, toasts). */
  short: string;
  /** A fake context (no-op actions) for the manager's preview and for tests: resolve the def with it to get the rows a viewer, a mod or a signed-out person sees. */
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  sample(who: PreviewAs): any;
  /** The name menu opened from the viewer list has no message: its custom entries get fewer placeholders. */
  viewerList?: boolean;
}

/** In the order the manager lists them (by heading, V110-N9). */
export const MENUS: MenuInfo[] = [
  { def: NAME_DEF, group: 'Chat', short: 'Names', sample: nameSample, viewerList: true },
  { def: MESSAGE_DEF, group: 'Chat', short: 'Messages', sample: messageSample },
  { def: EMOTE_DEF, group: 'Chat', short: 'Emotes', sample: emoteSample },
  { def: MORE_DEF, group: 'Chat', short: 'Chat header', sample: moreSample },
  { def: WHISPER_DEF, group: 'Chat', short: 'Whispers', sample: whisperSample },
  { def: FEED_NAME_DEF, group: 'Feeds', short: 'Names', sample: feedNameSample },
  { def: FEED_MESSAGE_DEF, group: 'Feeds', short: 'Messages', sample: feedMessageSample },
  { def: TAB_DEF, group: 'Tabs', short: 'Tabs', sample: tabSample },
  { def: GROUP_DEF, group: 'Tabs', short: 'Tab groups', sample: groupSample },
  { def: BAR_DEF, group: 'Tabs', short: 'Tab bar', sample: barSample },
  { def: TILE_DEF, group: 'Tabs', short: 'Tiles', sample: tileSample },
  { def: PLAYER_DEF, group: 'Watching', short: 'Players', sample: playerSample },
  { def: CARD_DEF, group: 'Watching', short: 'Channel cards', sample: cardSample },
  { def: CLIP_DEF, group: 'Watching', short: 'Clips', sample: clipSample },
];

/** The list as the manager draws it: each heading with its menus. */
export const MENUS_BY_GROUP = MENU_GROUPS.map((group) => ({ group, menus: MENUS.filter((m) => m.group === group) }));

export const MENU_DEFS: AnyDef[] = MENUS.map((m) => m.def);

export const menuInfo = (id: MenuId | string): MenuInfo | undefined => MENUS.find((m) => m.def.id === id);
export const defOf = (id: MenuId | string): AnyDef | undefined => menuInfo(id)?.def;

/** The fake context for `id`, as `who`. */
export function sampleCtx(id: MenuId | string, who: PreviewAs): unknown {
  return menuInfo(id)?.sample(who);
}

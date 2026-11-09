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

export interface MenuInfo {
  def: AnyDef;
  /** A fake context (no-op actions) for the manager's preview and for tests: resolve the def with it to get the rows a viewer, a mod or a signed-out person sees. */
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  sample(who: PreviewAs): any;
  /** The name menu opened from the viewer list has no message: its custom entries get fewer placeholders. */
  viewerList?: boolean;
}

/** In the order the manager lists them. */
export const MENUS: MenuInfo[] = [
  { def: NAME_DEF, sample: nameSample, viewerList: true },
  { def: MESSAGE_DEF, sample: messageSample },
  { def: EMOTE_DEF, sample: emoteSample },
  { def: MORE_DEF, sample: moreSample },
  { def: FEED_NAME_DEF, sample: feedNameSample },
  { def: FEED_MESSAGE_DEF, sample: feedMessageSample },
  { def: TAB_DEF, sample: tabSample },
  { def: TILE_DEF, sample: tileSample },
  { def: BAR_DEF, sample: barSample },
  { def: GROUP_DEF, sample: groupSample },
  { def: PLAYER_DEF, sample: playerSample },
  { def: CARD_DEF, sample: cardSample },
  { def: WHISPER_DEF, sample: whisperSample },
  { def: CLIP_DEF, sample: clipSample },
];

export const MENU_DEFS: AnyDef[] = MENUS.map((m) => m.def);

export const menuInfo = (id: MenuId | string): MenuInfo | undefined => MENUS.find((m) => m.def.id === id);
export const defOf = (id: MenuId | string): AnyDef | undefined => menuInfo(id)?.def;

/** The fake context for `id`, as `who`. */
export function sampleCtx(id: MenuId | string, who: PreviewAs): unknown {
  return menuInfo(id)?.sample(who);
}

// The tab bar's place and height, and which tabs it leaves out (v1.04 batch 2b): F03 a
// narrow window moves top tabs out of the title bar onto a row of their own, F16 a bar
// across the top or bottom shows up to N rows (or all), F17 "Only live tabs".

import type { Tab } from '../../ipc/bindings/Tab';
import type { TabBarSide } from '../../ipc/bindings/TabBarSide';
import type { TabRows } from '../../ipc/bindings/TabRows';

/** F03 Below this window width (CSS px) top tabs get their own row under the title bar. */
export const NARROW = 900;

/** `chrome`: in the title bar; `row`: a full-width row under it; `above`: over the chat
 * column inside the tab (V110-U52); else the side bar. */
export type BarPlace = 'chrome' | 'row' | 'above' | Exclude<TabBarSide, 'top' | 'aboveChat'>;

/** F03 Where the bar goes. A width of 0 (not measured yet) counts as wide. V110-U52 "Above
 * chat" is `above` while `aboveChat` says it applies, else the normal top place. */
export function barPlace(side: TabBarSide, width: number, aboveChat = false): BarPlace {
  if (side === 'aboveChat' && aboveChat) return 'above';
  if (side !== 'top' && side !== 'aboveChat') return side;
  return width > 0 && width < NARROW ? 'row' : 'chrome';
}

/** What V110-U52 needs to know about a tab: its tiles and how the one tile's chat sits. */
export interface ChatBeside {
  /** Tiles in the tab. */
  tiles: number;
  feed: boolean;
  /** The tile shows its stream and its chat, the chat not in its own window. */
  stream: boolean;
  chat: boolean;
  chatPopped: boolean;
  /** Where the chat sits (the tile's, else Settings'). */
  side: 'left' | 'right' | 'top' | 'bottom';
}

/** V110-U52 Whether the chat is beside the stream: one tile with both, the chat left or
 * right. The measured turns (too narrow, mod view, arranged by hand) come on top of this. */
export function chatBeside(i: ChatBeside): boolean {
  return i.tiles === 1 && !i.feed && i.stream && i.chat && !i.chatPopped && (i.side === 'left' || i.side === 'right');
}

/** A box on screen in CSS px. */
export interface Rect {
  x: number;
  y: number;
  w: number;
  h: number;
}

/** V110-F11 The bar as it was when the tab on screen changed under the pointer: over a chat
 * column (`above`) or at the top, and its box. Kept until the pointer leaves that box. */
export interface BarHold {
  above: boolean;
  rect: Rect;
}

/** V110-F11 How "Above chat" draws the bar now. `above`: the window leaves the top free (no
 * title-bar or row bar). `own`: the tile on screen draws the bar over its chat. `float`: the
 * window draws it fixed at this box instead. `spacer`: the tile's chat keeps this many px free
 * at its top, so it doesn't move once the hold ends and the tile draws its own bar there. */
export interface AboveBar {
  above: boolean;
  own: boolean;
  float: Rect | null;
  spacer: number;
}

/** V110-F11 Where "Above chat" puts the bar for the tab on screen. `applies`: the tab qualifies
 * (`AboveChat.applies`). While held, the bar stays where it was and the tab under it already
 * switches: held at the top it stays there; held over a chat it floats at the same box. */
export function aboveBar(applies: boolean, hold: BarHold | null): AboveBar {
  if (!hold) return { above: applies, own: applies, float: null, spacer: 0 };
  if (!hold.above) return { above: false, own: false, float: null, spacer: 0 };
  return { above: true, own: false, float: hold.rect, spacer: applies ? hold.rect.h : 0 };
}

/** V110-F11 A hold starts when the tab on screen changes while the pointer is on the bar (a
 * wheel notch, a click, a close), with "Above chat" set and the bar in view. */
export function startsHold(side: TabBarSide, hovered: boolean, rect: Rect | null): boolean {
  return side === 'aboveChat' && hovered && !!rect && rect.w > 0 && rect.h > 0;
}

/** V110-F11 The point is still on the held bar (edges count). */
export function inHold(rect: Rect, x: number, y: number): boolean {
  return x >= rect.x && x <= rect.x + rect.w && y >= rect.y && y <= rect.y + rect.h;
}

/** F16 "All" still leaves this much of the window's height to the tab under the bar. */
export const ALL_KEEP = 160;

/** F16 The CSS `max-height` of a bar across the top or bottom: whole rows of `rowH` px, or,
 * for "All", the window minus `ALL_KEEP`. */
export function barMaxHeight(mode: TabRows, rows: number, rowH: number): string {
  if (mode === 'all') return `calc(100vh - ${ALL_KEEP}px)`;
  return `${Math.max(1, Math.round(rows)) * rowH}px`;
}

/** What `hiddenTabs` needs to know about live channels: checked ones, by channel id. */
export type LiveById = Record<string, { stream: unknown | null } | undefined>;

/** F17 With "Only live tabs" on, the tabs to leave out of the bar: every channel in them
 * is offline (a rerun counts as live; one the check didn't find, e.g. a banned channel,
 * counts as offline). Always kept: the tabs in `keep` (the one on screen, one playing out
 * of sight, popped-out ones) and feed tabs (no channel). Nothing hides before the first
 * live check. */
export function hiddenTabs(list: readonly Tab[], on: boolean, keep: Iterable<string | null>, live: LiveById): Set<string> {
  const out = new Set<string>();
  if (!on || !Object.keys(live).length) return out;
  const kept = new Set(keep);
  for (const tab of list) {
    if (kept.has(tab.id)) continue;
    const tiles = tab.columns.flatMap((c) => c.tiles);
    if (!tiles.length || tiles.some((t) => t.feed || !t.channel.id)) continue;
    if (tiles.every((t) => !live[t.channel.id]?.stream)) out.add(tab.id);
  }
  return out;
}

/** V106-26 Width of a left tab bar: up to the header's divider (`edge`, its right side in
 * CSS px from the bar's left, 0 = not measured yet), so the two lines meet. Other sides keep the tab size's. */
export function sideBarWidth(side: TabBarSide, edge: number): number | undefined {
  return side === 'left' && edge > 0 ? edge - 1 : undefined;
}

/** V108-F59d The shown tab and the "+" below it can both be in view in a bar `view` px tall
 * (tops and bottoms in the same coordinates): the bar then scrolls to the "+" too. */
export function plusFits(tabTop: number, plusBottom: number, view: number): boolean {
  return plusBottom >= tabTop && plusBottom - tabTop <= view;
}

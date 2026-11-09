// The tab bar's place and height, and which tabs it leaves out (v1.04 batch 2b): F03 a
// narrow window moves top tabs out of the title bar onto a row of their own, F16 a bar
// across the top or bottom shows up to N rows (or all), F17 "Only live tabs".

import type { Tab } from '../../ipc/bindings/Tab';
import type { TabBarSide } from '../../ipc/bindings/TabBarSide';
import type { TabRows } from '../../ipc/bindings/TabRows';

/** F03 Below this window width (CSS px) top tabs get their own row under the title bar. */
export const NARROW = 900;

/** `chrome`: in the title bar; `row`: a full-width row under it; `above`: the one "Above chat"
 * bar (V110-U54 `AboveBar`, placed by `aboveSpot`); else the side bar. */
export type BarPlace = 'chrome' | 'row' | 'above' | Exclude<TabBarSide, 'top' | 'aboveChat'>;

/** F03 Where the bar goes. A width of 0 (not measured yet) counts as wide. */
export function barPlace(side: TabBarSide, width: number): BarPlace {
  if (side === 'aboveChat') return 'above';
  if (side !== 'top') return side;
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
  /** V110-F38 Theatre mode is on for this tab: no tab bar anywhere (V108-N14). */
  theatre: boolean;
  /** Where the chat sits (the tile's, else Settings'). */
  side: 'left' | 'right' | 'top' | 'bottom';
}

/** V110-U52 Whether the chat is beside the stream: one tile with both, the chat left or
 * right. The measured turns (too narrow, mod view, arranged by hand) come on top of this. */
export function chatBeside(i: ChatBeside): boolean {
  return !i.theatre && i.tiles === 1 && !i.feed && i.stream && i.chat && !i.chatPopped && (i.side === 'left' || i.side === 'right');
}

/** A box on screen in CSS px. */
export interface Rect {
  x: number;
  y: number;
  w: number;
  h: number;
}

/** V110-U54 Where the one "Above chat" bar is: in the title bar or the narrow-window row
 * (where Top puts it), in the slot at the top of the shown tile's chat column, fixed at a held
 * box (`float`), or hidden (theatre). */
export type AboveSpot = 'chrome' | 'row' | 'slot' | 'float' | 'none';

/** V110-F11 The bar as it was when the tab on screen changed under the pointer: at the top or
 * over a chat column, and its box. Kept until the pointer leaves that box. */
export interface AboveHold {
  at: 'top' | 'chat';
  rect: Rect;
}

export interface SpotFor {
  /** V110-F38 Theatre mode is on for the tab on screen. */
  theatre: boolean;
  /** The window's width (F03 row below `NARROW`). */
  width: number;
  /** The tab on screen qualifies (`AboveChat.applies`) and its tile has drawn its slot. */
  slot: boolean;
  held: AboveHold['at'] | null;
  /** While held over a chat: the new tab's slot is where the held box is. */
  lined: boolean;
}

/** V110-U54 (V110-F11) The bar never moves under the pointer: held at the top it stays there;
 * held over a chat it stays at that box, in the new tab's slot when that lines up, else fixed
 * there. Not held, it goes to the slot when there is one, else to the top. */
export function aboveSpot(i: SpotFor): AboveSpot {
  if (i.theatre) return 'none';
  const top = i.width > 0 && i.width < NARROW ? 'row' : 'chrome';
  if (i.held === 'top') return top;
  if (i.held === 'chat') return i.slot && i.lined ? 'slot' : 'float';
  return i.slot ? 'slot' : top;
}

/** V110-U54 What a hold at `spot` keeps: the top, or a box over a chat. */
export function holdAt(spot: AboveSpot): AboveHold['at'] | null {
  if (spot === 'chrome' || spot === 'row') return 'top';
  return spot === 'none' ? null : 'chat';
}

/** V110-U54 Two boxes are the same place on screen, give or take `tol` px (a slot drawn by
 * another tile, rounding). */
export function linesUp(a: Rect, b: Rect, tol = 2): boolean {
  return Math.abs(a.x - b.x) <= tol && Math.abs(a.y - b.y) <= tol && Math.abs(a.w - b.w) <= tol;
}

/** V110-F42 Above chat leaves the chat this much: a column narrower than `ABOVE_NARROW` px or a
 * window shorter than `ABOVE_SHORT` px gets a bar of one row (it scrolls within it, V110-U51). */
export const ABOVE_NARROW = 260;
export const ABOVE_SHORT = 560;
export function aboveOneRow(colWidth: number, winHeight: number): boolean {
  return (colWidth > 0 && colWidth < ABOVE_NARROW) || (winHeight > 0 && winHeight < ABOVE_SHORT);
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

/** V110-F40 The side "Above chat" goes back to when it is turned off: the one stored when it
 * was turned on, if it is a real side, else Top. */
export function sideBeforeAbove(stored: unknown): Exclude<TabBarSide, 'aboveChat'> {
  return stored === 'bottom' || stored === 'left' || stored === 'right' ? stored : 'top';
}

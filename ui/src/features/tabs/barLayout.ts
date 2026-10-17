// The tab bar's place and height, and which tabs it leaves out (v1.04 batch 2b): F03 a
// narrow window moves top tabs out of the title bar onto a row of their own, F16 a bar
// across the top or bottom shows up to N rows (or all), F17 "Only live tabs".

import type { Tab } from '../../ipc/bindings/Tab';
import type { TabBarSide } from '../../ipc/bindings/TabBarSide';
import type { TabRows } from '../../ipc/bindings/TabRows';

/** F03 Below this window width (CSS px) top tabs get their own row under the title bar. */
export const NARROW = 900;

/** `chrome`: in the title bar; `row`: a full-width row under it; else the side bar. */
export type BarPlace = 'chrome' | 'row' | Exclude<TabBarSide, 'top'>;

/** F03 Where the bar goes. A width of 0 (not measured yet) counts as wide. */
export function barPlace(side: TabBarSide, width: number): BarPlace {
  if (side !== 'top') return side;
  return width > 0 && width < NARROW ? 'row' : 'chrome';
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

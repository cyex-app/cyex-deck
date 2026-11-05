// V109-B02 cyex badges: which show, their tooltips, and what the cosmetics area says. At most two
// show: Founder or Supporter, then the Supporter+ badge.

import type { CyexMe } from '../../ipc/bindings/CyexMe.ts';
import type { CyexPics } from '../../ipc/bindings/CyexPics.ts';

export const SHOP = 'https://cyex.app/deck/cosmetics';
export const UPLOAD = 'https://cyex.app/deck/cosmetics#badge';

export type Main = 'founder' | 'supporter';
export type Choice = Main | 'none';

/** Chat badges of the cyex set carry ids like `cyex:founder-37`. */
export function isCyexBadge(id: string): boolean {
  return id.startsWith('cyex:');
}

export function badgeTitle(kind: string, number?: number | null): string {
  if (kind === 'founder') return number ? `cyex founder #${number}` : 'cyex founder';
  if (kind === 'supporter') return 'cyex supporter';
  return 'cyex supporter+';
}

/** A founder counts only with its number. */
export function owns(me: CyexMe, kind: string): boolean {
  if (kind === 'custom') return me.plus.active;
  return me.badges.some((b) => b.id === kind && (kind !== 'founder' || b.number != null));
}

export function founderNumber(me: CyexMe): number | null {
  return me.badges.find((b) => b.id === 'founder')?.number ?? null;
}

export function nothingOwned(me: CyexMe): boolean {
  return !owns(me, 'founder') && !owns(me, 'supporter') && !owns(me, 'custom');
}

export function choiceOf(me: CyexMe): Choice {
  const m = me.equipped.main;
  return m === 'founder' || m === 'supporter' ? m : 'none';
}

export interface ShownBadge {
  key: string;
  title: string;
  pics: CyexPics;
}

/** What chat shows for the account now (`equipped.shown`), two at most, in order. */
export function shownBadges(me: CyexMe, founder: CyexPics, supporter: CyexPics): ShownBadge[] {
  const out: ShownBadge[] = [];
  for (const id of me.equipped.shown) {
    if (out.length === 2) break;
    if (id === 'founder' && owns(me, 'founder')) out.push({ key: id, title: badgeTitle(id, founderNumber(me)), pics: founder });
    else if (id === 'supporter' && owns(me, 'supporter')) out.push({ key: id, title: badgeTitle(id), pics: supporter });
    else if (id === 'custom' && me.plus.live) out.push({ key: id, title: badgeTitle(id), pics: me.plus.live });
  }
  return out;
}

/** The Supporter+ picture's state when none is live. */
export function plusNote(me: CyexMe): string {
  const s = me.plus.badge?.status ?? '';
  if (s === 'pending' || s === 'review' || s === 'waiting') return 'Waiting for review';
  if (s === 'rejected') return 'Not approved. Upload another one.';
  return 'No badge yet';
}

/** The usercard's short line from the badge titles chat shows, e.g. `cyex founder #37 · supporter+`. */
export function usercardLine(titles: string[]): string {
  const cyex = titles.filter((t) => t.startsWith('cyex '));
  return cyex.map((t, i) => (i === 0 ? t : t.slice(5))).join(' · ');
}

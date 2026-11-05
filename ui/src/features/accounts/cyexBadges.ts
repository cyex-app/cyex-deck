// V109-B02 cyex badges: which show, their tooltips, and what the cosmetics area says. At most two
// show: Founder or Supporter, then the Supporter+ badge.

import type { CyexMe } from '../../ipc/bindings/CyexMe.ts';
import type { CyexPics } from '../../ipc/bindings/CyexPics.ts';
import { formatDate, formatNumber } from '../../lib/format.ts';

/** V109-B03 The store tab of cyex.app's cosmetics page, and its own-badge tab. */
export const STORE = 'https://cyex.app/deck/cosmetics#store';
export const BADGE_PAGE = 'https://cyex.app/deck/cosmetics#badge';

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

/** V109-B03 The badge picker: owned ones can be picked, the rest show but stay off. */
export interface BadgeOption {
  value: Choice;
  label: string;
  owned: boolean;
}

export function badgeOptions(me: CyexMe): BadgeOption[] {
  const n = founderNumber(me);
  const founder = owns(me, 'founder');
  return [
    { value: 'founder', label: founder && n ? `Founder #${formatNumber(n)}` : 'Founder', owned: founder },
    { value: 'supporter', label: 'Supporter', owned: owns(me, 'supporter') },
    { value: 'none', label: 'None', owned: true },
  ];
}

/** `2026-12-05T12:00:00Z` → `2026/12/05` (local day); unreadable text keeps its first 10 chars. */
export function day(iso: string): string {
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? iso.slice(0, 10).replaceAll('-', '/') : formatDate(d);
}

/** V109-B03 Where the Supporter+ plan and its picture stand. `never`: no row at all. */
export type PlusKind = 'never' | 'ended' | 'live' | 'review' | 'rejected' | 'empty';

export interface PlusState {
  kind: PlusKind;
  /** The plan line under the label: `Renews 2026/12/05`, `Until …`, `Ended …`. */
  plan: string;
  /** What the picture is doing when none shows. */
  note: string;
  /** The one link: change or upload the picture (#badge), or renew (#store). */
  link: { label: string; url: string } | null;
}

export function plusState(me: CyexMe): PlusState {
  const p = me.plus;
  if (!p.active) {
    if (!p.until && !p.badge && !p.live) return { kind: 'never', plan: '', note: '', link: null };
    return { kind: 'ended', plan: p.until ? `Ended ${day(p.until)}` : 'Ended', note: 'Not in chat while the plan is off', link: { label: 'Renew', url: STORE } };
  }
  const plan = p.renewsAt ? `Renews ${day(p.renewsAt)}` : p.until ? `Until ${day(p.until)}` : '';
  const change = { label: 'Change picture', url: BADGE_PAGE };
  const upload = { label: 'Upload a picture', url: BADGE_PAGE };
  if (p.live) return { kind: 'live', plan, note: '', link: change };
  const s = p.badge?.status ?? '';
  if (s === 'pending' || s === 'review' || s === 'waiting') return { kind: 'review', plan, note: 'Waiting for review', link: change };
  if (s === 'rejected') return { kind: 'rejected', plan, note: 'Not approved. Try another one.', link: upload };
  return { kind: 'empty', plan, note: 'No picture yet', link: upload };
}

/** V109-B03 Nothing to set: no badge and never a Supporter+ plan. The store is the one action. */
export function nothingToSet(me: CyexMe): boolean {
  return nothingOwned(me) && plusState(me).kind === 'never';
}

/** The usercard's short line from the badge titles chat shows, e.g. `cyex founder #37 · supporter+`. */
export function usercardLine(titles: string[]): string {
  const cyex = titles.filter((t) => t.startsWith('cyex '));
  return cyex.map((t, i) => (i === 0 ? t : t.slice(5))).join(' · ');
}

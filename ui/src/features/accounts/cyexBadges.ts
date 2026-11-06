// V109-B02 cyex badges: which show, their tooltips, and what the cosmetics area says. At most two
// show: Founder or Supporter, then the Supporter+ badge.

import type { CyexMe } from '../../ipc/bindings/CyexMe.ts';
import type { CyexPics } from '../../ipc/bindings/CyexPics.ts';
import type { CyexView } from '../../ipc/bindings/CyexView.ts';
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

/** V109-B04 Slot 1 holds one one-time badge. A new kind (e.g. a streak badge) is one more entry
 *  here; its picture comes from `pics` of the view. */
export interface Slot1Kind {
  id: Main;
  label: (me: CyexMe) => string;
  pics: (view: CyexView) => CyexPics | null;
}

export const SLOT1_KINDS: readonly Slot1Kind[] = [
  {
    id: 'founder',
    label: (me) => {
      const n = founderNumber(me);
      return n ? `Founder #${formatNumber(n)}` : 'Founder';
    },
    pics: (v) => (v.state === 'ready' ? v.founder : null),
  },
  { id: 'supporter', label: () => 'Supporter', pics: (v) => (v.state === 'ready' ? v.supporter : null) },
];

export interface Slot1Option {
  id: Main;
  label: string;
  owned: boolean;
}

export function slot1Options(me: CyexMe): Slot1Option[] {
  return SLOT1_KINDS.map((k) => ({ id: k.id, label: k.label(me), owned: owns(me, k.id) }));
}

/** The equipped slot-1 badge, if the account still owns it. */
export function slot1Current(me: CyexMe): Slot1Option | null {
  const c = choiceOf(me);
  return slot1Options(me).find((o) => o.id === c && o.owned) ?? null;
}

/** V109-B05 A card in slot 1's picker (the shared cosmetics grid takes this shape). */
export interface PickerRow {
  name: string;
  animated: boolean;
  usable: boolean;
  item: { def: { id: string }; images: readonly string[]; stills: readonly string[] | null };
}

const picsList = (p: CyexPics | null) => (p ? [p['1x'], p['2x'] || p['1x'], p['4x'] || p['2x'] || p['1x'], p['4x'] || p['2x'] || p['1x']] : ['', '', '', '']);

/** V109-B05 Slot 1's cards: the badges this account owns, in the kinds list's order. */
export function slot1Rows(me: CyexMe, view: CyexView): PickerRow[] {
  return SLOT1_KINDS.filter((k) => owns(me, k.id)).map((k) => ({
    name: k.label(me),
    animated: false,
    usable: true,
    item: { def: { id: k.id }, images: picsList(k.pics(view)), stills: null },
  }));
}

/** Search by name; empty text keeps all. */
export function filterRows<T extends { name: string }>(rows: readonly T[], text: string): T[] {
  const t = text.trim().toLowerCase();
  return t ? rows.filter((r) => r.name.toLowerCase().includes(t)) : [...rows];
}

/** Some slot-1 kind isn't owned: the picker keeps one quiet store link. */
export function slot1Missing(me: CyexMe): boolean {
  return SLOT1_KINDS.some((k) => !owns(me, k.id));
}

/** V109-B05 Slot 1's choice: what's worn, else the one remembered while it's switched off. */
export function slot1Choice(me: CyexMe, remembered: string | null): Main | null {
  const worn = slot1Current(me)?.id;
  if (worn) return worn;
  const r = SLOT1_KINDS.find((k) => k.id === remembered);
  return r && owns(me, r.id) ? r.id : null;
}

/** V109-B05 The equip call for a pick in slot 1 (null: nothing to do). Picking turns the slot on. */
export function slot1Pick(me: CyexMe, id: string | null): { main: Main | null } | null {
  const main = (SLOT1_KINDS.find((k) => k.id === id && owns(me, k.id))?.id ?? null) as Main | null;
  if (id !== null && !main) return null;
  return main === (slot1Current(me)?.id ?? null) ? null : { main };
}

/** The slot-1 choice kept per account while the slot is off (the equip API has only `main`). */
const REMEMBER_KEY = 'cyex.slot1.';

export function readSlot1(account: string): string | null {
  try {
    return localStorage.getItem(REMEMBER_KEY + account);
  } catch {
    return null;
  }
}

export function saveSlot1(account: string, id: string | null): void {
  try {
    if (id) localStorage.setItem(REMEMBER_KEY + account, id);
    else localStorage.removeItem(REMEMBER_KEY + account);
  } catch {
    // Not kept: switching on again starts from None.
  }
}

/** V109-B05 The Supporter+ panel: the picture, its review state, the plan, the one link. */
export interface PlusPanel {
  picture: CyexPics | null;
  review: string;
  plan: string;
  link: { label: string; url: string } | null;
}

export function plusPanel(me: CyexMe): PlusPanel {
  const s = plusState(me);
  const p = me.plus;
  const status = p.badge?.status ?? '';
  const review = p.live
    ? 'Approved'
    : s.kind === 'review'
      ? 'In review'
      : s.kind === 'rejected' || status === 'rejected'
        ? 'Not approved. Try another one.'
        : 'No picture yet';
  const plan = !p.active
    ? s.plan
    : p.renewsAt
      ? `Active, renews ${day(p.renewsAt)}`
      : p.until
        ? `Active until ${day(p.until)}`
        : 'Active';
  return { picture: p.live, review, plan, link: s.link };
}

/** V109-B04 What opening slot 2 found: the plan is on (open), off (locked), or the check failed. */
export type PlusCheck =
  | { kind: 'open' | 'locked'; /** A new view when it was fetched, null from the cache. */ fetched: CyexView | null }
  | { kind: 'error'; message: string };

export const PLUS_FRESH_MS = 30_000;
const CHECK_FAILED = 'Couldn’t check Supporter+. Try again.';

/** V109-B04 Slot 2 opens only after a fresh look at the plan; a look from the last 30 s counts. */
export function plusGate(fetch: () => Promise<CyexView>, now: () => number = Date.now) {
  let last: CyexMe | null = null;
  let at = -Infinity;
  let pending: Promise<PlusCheck> | null = null;
  const keep = (v: CyexView) => {
    if (v.state !== 'ready') return;
    last = v.me;
    at = now();
  };
  const fresh = () => !!last && now() - at < PLUS_FRESH_MS;
  return {
    /** A view loaded elsewhere (the editor opening) counts as a look. */
    seen: keep,
    fresh,
    check(): Promise<PlusCheck> {
      if (fresh()) return Promise.resolve({ kind: last!.plus.active ? 'open' : 'locked', fetched: null });
      if (pending) return pending;
      const p: Promise<PlusCheck> = fetch().then(
        (v) => {
          if (v.state !== 'ready') return { kind: 'error', message: v.state === 'noToken' ? 'Sign in again to check Supporter+.' : CHECK_FAILED };
          keep(v);
          return { kind: v.me.plus.active ? 'open' : 'locked', fetched: v };
        },
        () => ({ kind: 'error', message: CHECK_FAILED }),
      );
      pending = p;
      void p.then(() => (pending = null));
      return p;
    },
  };
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

// V109-B02 cyex badges: which show, their tooltips, and what the cosmetics area says. At most two
// show: one one-time badge, then the Supporter+ badge.
// V109-B10 The one-time badges are the catalog's (`/badges/v1/badges`, store order): founder,
// supporter and whatever the store adds. A numbered one shows "#n" whatever its kind.

import type { CyexKind } from '../../ipc/bindings/CyexKind.ts';
import type { CyexMe } from '../../ipc/bindings/CyexMe.ts';
import type { CyexPics } from '../../ipc/bindings/CyexPics.ts';
import type { CyexHistory } from '../../ipc/bindings/CyexHistory.ts';
import type { CyexView } from '../../ipc/bindings/CyexView.ts';
import { formatDate, formatNumber } from '../../lib/format.ts';

/** V109-B03 The store tab of cyex.app's cosmetics page, and its own-badge tab. */
export const STORE = 'https://cyex.app/deck/cosmetics#store';
export const BADGE_PAGE = 'https://cyex.app/deck/cosmetics#badge';

/** A one-time badge id (`founder`, `supporter`, `streak-1y`, …). */
export type Main = string;

/** Chat badges of the cyex set carry ids like `cyex:founder-37`. */
export function isCyexBadge(id: string): boolean {
  return id.startsWith('cyex:');
}

/** `streak-1y` → `cyex streak 1y`, for a kind the catalog doesn't name (yet). */
function fallbackTitle(kind: string): string {
  if (kind === 'custom') return 'cyex supporter+';
  return `cyex ${kind.replaceAll('-', ' ')}`;
}

/** The tooltip: the catalog's title (or a readable one), plus `#n` on a numbered badge. */
export function badgeTitle(kind: string, number?: number | null, named?: string | null): string {
  const base = named?.trim() || fallbackTitle(kind);
  return number && kind !== 'custom' ? `${base} #${number}` : base;
}

/** The kind ids the app takes: like the store's (lowercase, digits, dashes). */
export function kindOk(id: string): boolean {
  return /^[a-z][a-z0-9-]{0,39}$/.test(id) && id !== 'custom';
}

/** A founder counts only with its number. */
export function owns(me: CyexMe, kind: string): boolean {
  if (kind === 'custom') return me.plus.active;
  return kindOk(kind) && me.badges.some((b) => b.id === kind && (kind !== 'founder' || b.number != null));
}

export function numberOf(me: CyexMe, kind: string): number | null {
  return me.badges.find((b) => b.id === kind)?.number ?? null;
}

/** No one-time badge and no Supporter+. */
export function nothingOwned(me: CyexMe): boolean {
  return !me.badges.some((b) => owns(me, b.id)) && !owns(me, 'custom');
}

export function choiceOf(me: CyexMe): Main | 'none' {
  const m = me.equipped.main;
  return m && kindOk(m) ? m : 'none';
}

/** V109-B10 The catalog's one-time badges, then any owned one it doesn't name yet. */
export function oneTimeKinds(me: CyexMe, view: CyexView): CyexKind[] {
  const kinds = view.state === 'ready' ? view.kinds.filter((k) => kindOk(k.id)) : [];
  const extra = me.badges
    .filter((b) => owns(me, b.id) && !kinds.some((k) => k.id === b.id))
    .map((b) => ({ id: b.id, title: fallbackTitle(b.id), images: { '1x': '', '2x': '', '4x': '' }, stills: null }));
  return [...kinds, ...extra.filter((k, i) => extra.findIndex((x) => x.id === k.id) === i)];
}

/** The card's name: `cyex founder` → `Founder`, with `#n` when numbered. */
export function kindLabel(me: CyexMe, k: CyexKind): string {
  const name = k.title.replace(/^cyex\s+/i, '').trim() || k.id;
  const n = numberOf(me, k.id);
  const cap = name[0].toUpperCase() + name.slice(1);
  return n ? `${cap} #${formatNumber(n)}` : cap;
}

export interface ShownBadge {
  key: string;
  title: string;
  pics: CyexPics;
}

/** What chat shows for the account now (`equipped.shown`), two at most, in order. */
export function shownBadges(me: CyexMe, view: CyexView): ShownBadge[] {
  const kinds = oneTimeKinds(me, view);
  const out: ShownBadge[] = [];
  for (const id of me.equipped.shown) {
    if (out.length === 2) break;
    if (id === 'custom') {
      if (me.plus.live) out.push({ key: id, title: badgeTitle(id), pics: me.plus.live });
      continue;
    }
    const k = kinds.find((x) => x.id === id);
    if (k && owns(me, id)) out.push({ key: id, title: badgeTitle(id, numberOf(me, id), k.title), pics: k.images });
  }
  return out;
}

/** The equipped slot-1 badge id, if the account still owns it. */
export function slot1Current(me: CyexMe): Main | null {
  const c = choiceOf(me);
  return c !== 'none' && owns(me, c) ? c : null;
}

/** V109-B05 A card in slot 1's picker (the shared cosmetics grid takes this shape). */
export interface PickerRow {
  name: string;
  sub: string;
  animated: boolean;
  usable: boolean;
  item: { def: { id: string }; images: readonly string[]; stills: readonly string[] | null };
}

const picsList = (p: CyexPics | null) => (p ? [p['1x'], p['2x'] || p['1x'], p['4x'] || p['2x'] || p['1x'], p['4x'] || p['2x'] || p['1x']] : ['', '', '', '']);

/** V109-B05/B10 Slot 1's cards: every one-time badge this account owns, in store order. */
export function slot1Rows(me: CyexMe, view: CyexView): PickerRow[] {
  return oneTimeKinds(me, view)
    .filter((k) => owns(me, k.id))
    .map((k) => ({
      name: kindLabel(me, k),
      sub: 'One-time',
      animated: !!k.stills,
      usable: true,
      item: { def: { id: k.id }, images: picsList(k.images), stills: k.stills ? picsList(k.stills) : null },
    }));
}

/** V109-B08 Search and the tiles switch show from this many cards (None included) on. */
export const SEARCH_FROM = 9;

/** V109-B08 The badge grid's height for `cells` cards at `width`: every row shows, four at most. */
export function gridHeight(cells: number, width: number, rows: boolean): number {
  const gap = 6;
  const [w, h] = rows ? [190, 44] : [72, 76];
  const columns = Math.max(1, Math.floor((width + gap) / (w + gap)));
  const lines = Math.min(4, Math.max(1, Math.ceil(cells / columns)));
  return lines * h + (lines - 1) * gap;
}

/** Search by name; empty text keeps all. */
export function filterRows<T extends { name: string }>(rows: readonly T[], text: string): T[] {
  const t = text.trim().toLowerCase();
  return t ? rows.filter((r) => r.name.toLowerCase().includes(t)) : [...rows];
}

/** Some catalog badge isn't owned: the section keeps one quiet store link. */
export function slot1Missing(me: CyexMe, view: CyexView): boolean {
  return oneTimeKinds(me, view).some((k) => !owns(me, k.id));
}

/** V109-B05 Slot 1's choice: what's worn, else the one remembered while it's switched off. */
export function slot1Choice(me: CyexMe, remembered: string | null): Main | null {
  const worn = slot1Current(me);
  if (worn) return worn;
  return remembered && owns(me, remembered) ? remembered : null;
}

/** V109-B05 The equip call for a pick in slot 1 (null: nothing to do). Picking turns the slot on. */
export function slot1Pick(me: CyexMe, id: string | null): { main: Main | null } | null {
  if (id !== null && !owns(me, id)) return null;
  return id === slot1Current(me) ? null : { main: id };
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
  /** V109-B06 Why the newest upload was turned down. */
  reason: string;
  plan: string;
  link: { label: string; url: string } | null;
}

const REVIEWING = ['pending', 'review', 'waiting'];
const TURNED_DOWN = ['rejected', 'removed'];

/** V109-B06 The newest upload's state comes first; the live picture keeps showing meanwhile. */
export function plusPanel(me: CyexMe): PlusPanel {
  const s = plusState(me);
  const p = me.plus;
  const status = p.badge?.status ?? '';
  let review = p.live ? 'Approved' : 'No picture yet';
  let reason = '';
  if (REVIEWING.includes(status)) review = p.live ? 'New picture in review' : 'In review';
  else if (TURNED_DOWN.includes(status)) {
    review = p.live ? 'New picture not approved' : 'Not approved';
    reason = p.badge?.reason?.trim() || 'Try another one.';
  }
  const plan = !p.active
    ? s.plan
    : p.renewsAt
      ? `Active, renews ${day(p.renewsAt)}`
      : p.until
        ? `Active until ${day(p.until)}`
        : 'Active';
  return { picture: p.live, review, reason, plan, link: s.link };
}

/** V109-B06 A line in the upload history. */
export interface UploadRow {
  id: number;
  pics: CyexPics | null;
  date: string;
  status: string;
  reason: string;
  live: boolean;
  restorable: boolean;
  /** V109-B07 Not approved: the picture is an owner preview, loaded through the app's session. */
  preview: string | null;
}

const STATUS: Record<string, string> = {
  approved: 'Approved',
  pending: 'In review',
  review: 'In review',
  waiting: 'In review',
  rejected: 'Not approved',
  removed: 'Removed',
  withdrawn: 'Withdrawn',
};

export function uploadRows(h: CyexHistory): UploadRow[] {
  return h.items.map((u) => {
    const live = u.id === h.live;
    return {
      id: u.id,
      pics: u.status === 'approved' ? u.images : null,
      date: day(u.uploadedAt),
      status: live ? 'Live' : (STATUS[u.status] ?? (u.status ? u.status[0].toUpperCase() + u.status.slice(1) : '')),
      reason: TURNED_DOWN.includes(u.status) ? (u.reason?.trim() ?? '') : '',
      live,
      restorable: u.restorable && !live,
      preview: u.status !== 'approved' && u.images ? u.images['2x'] || u.images['1x'] || null : null,
    };
  });
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
  /** The plan line under the label: `Renews 2026/12/05`, `Free until …`, `Ended …`. */
  plan: string;
  /** What the picture is doing when none shows. */
  note: string;
  /** The one link: change or upload the picture (#badge), or renew (#store). */
  link: { label: string; url: string } | null;
}

export function plusState(me: CyexMe): PlusState {
  const p = me.plus;
  if (!p.active) {
    // V109-F76 The server sends `badge: { status: 'none' }` to someone who never had a picture.
    const picture = !!p.badge && p.badge.status !== 'none';
    if (!p.until && !picture && !p.live) return { kind: 'never', plan: '', note: '', link: null };
    return { kind: 'ended', plan: p.until ? `Ended ${day(p.until)}` : 'Ended', note: 'Not in chat while the plan is off', link: { label: 'Renew', url: STORE } };
  }
  const plan = p.renewsAt ? `Renews ${day(p.renewsAt)}` : p.until ? `Free until ${day(p.until)}` : '';
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

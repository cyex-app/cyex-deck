// V108-C40 Whether the chat box is locked for this account, and what the lock says: a timeout
// counting down, a ban with its reason and the unban request state, or a warning to
// acknowledge. Pure, so the rules are tested apart from the component.

import type { ChatStanding } from '../../ipc/bindings/ChatStanding';
import type { ChatWarning } from '../../ipc/bindings/ChatWarning';
import type { PastUnban } from '../../ipc/bindings/PastUnban';
import type { Restriction } from '../../ipc/bindings/Restriction';
import { formatDate, formatTime } from '../../lib/format.ts';
import { clock } from '../player/time.ts';

/** Same as Rust's `REQUEST_MAX`. */
export const REQUEST_MAX = 500;
/** A read that started this soon after IRC's word may not know of it yet. */
export const READ_LAG_MS = 3000;

/** What IRC said, with when it arrived (ms). */
export interface Said {
  how: Restriction;
  at: number;
}

export type Unban =
  /** The channel takes no requests (or didn't say). */
  | { state: 'off' }
  /** Open from `at` (ms). */
  | { state: 'wait'; at: number }
  | { state: 'ready'; last: PastUnban | null }
  | { state: 'pending'; request: PastUnban }
  /** Twitch says no more requests here; `last` when it's readable. */
  | { state: 'closed'; last: PastUnban | null };

export type Lock =
  | { kind: 'timeout'; until: number; reason: string }
  | { kind: 'ban'; reason: string; since: number | null; unban: Unban }
  | { kind: 'warn'; warning: ChatWarning };

export interface LockInput {
  standing: ChatStanding | null;
  /** When the standing's read started (ms), 0 before the first. */
  readAt: number;
  said: Said | null;
  warning: ChatWarning | null;
  now: number;
}

const time = (s: string | null | undefined): number | null => {
  const t = s ? Date.parse(s) : NaN;
  return Number.isNaN(t) ? null : t;
};

/** IRC's word counts until a read that started after it says otherwise. */
export function saidCounts(said: Said | null, readAt: number): boolean {
  return !!said && said.at + READ_LAG_MS > readAt;
}

export function lockOf({ standing, readAt, said, warning, now }: LockInput): Lock | null {
  const ban = standing?.ban ?? null;
  const fresh = saidCounts(said, readAt) ? said : null;
  const reason = ban?.reason ?? '';
  if (fresh?.how.kind === 'banned' || (ban && ban.until === null)) {
    // A ban the read doesn't know yet has no reason or time; it fills in with the next read.
    const known = ban && ban.until === null;
    return {
      kind: 'ban',
      reason: known ? reason : '',
      since: known ? time(ban.since) : (fresh?.at ?? null),
      unban: unbanOf(known ? standing : null, known ? time(ban.since) : null, now, readAt),
    };
  }
  const until = fresh?.how.kind === 'timedOut' ? fresh.at + fresh.how.secs * 1000 : time(ban?.until);
  if (until !== null && until > now) {
    return { kind: 'timeout', until, reason: fresh?.how.kind === 'timedOut' && !ban ? '' : reason };
  }
  // V108-C40g A read never shows a warning, so IRC's word holds until it's acknowledged.
  const warned = warning ?? (said?.how.kind === 'warned' ? { reason: '', rules: [] } : null);
  if (warned) return { kind: 'warn', warning: warned };
  return null;
}

/** Whether an unban request can go out now, and if not, why or when. `readAt`: when the
 *  standing's read started; Twitch's "no" from before the wait ended is about the wait. */
export function unbanOf(standing: ChatStanding | null, since: number | null, now: number, readAt = now): Unban {
  const rules = standing?.unban;
  if (!standing || !rules?.enabled) return { state: 'off' };
  const last = standing.request;
  if (last?.status === 'PENDING') return { state: 'pending', request: last };
  if (standing.canRequest === true) return { state: 'ready', last };
  const at = since === null ? null : since + rules.cooldownMinutes * 60_000;
  if (at !== null && at > now) return { state: 'wait', at };
  // V108-C40f The wait just ran out and the next read is on its way: ready, not "can't".
  if (standing.canRequest === false && (at === null || readAt >= at)) return { state: 'closed', last };
  return { state: 'ready', last };
}

/** "4:59", "1:02:03"; from a day on the date and time it ends. */
export function left(until: number, now: number): string {
  const ms = until - now;
  if (ms >= 86_400_000) {
    const d = new Date(until);
    return `${formatDate(d)} ${formatTime(d)}`;
  }
  return clock(Math.ceil(ms / 1000));
}

/** V108-F57a The lock's words without the ticking part: what a screen reader hears. */
export function title(lock: Lock): string {
  switch (lock.kind) {
    case 'timeout':
      return 'Timed out';
    case 'ban':
      return 'You’re banned here.';
    case 'warn':
      return 'You got a warning.';
  }
}

/** V108-F57a The time left on a timeout, else ''. It ticks, so it stays out of the status. */
export function ticking(lock: Lock, now: number): string {
  return lock.kind === 'timeout' ? left(lock.until, now) : '';
}

/** V108-F57b When a lock ends, the box takes focus back only if it had it or nothing has it. */
export function refocusBox(hadFocus: boolean, activeIsBody: boolean): boolean {
  return hadFocus || activeIsBody;
}

/** The lock's one line. */
export function headline(lock: Lock, now: number): string {
  switch (lock.kind) {
    case 'timeout':
      return `Timed out · ${left(lock.until, now)}`;
    case 'ban':
      return 'You’re banned here.';
    case 'warn':
      return 'You got a warning.';
  }
}

/** The unban line under a ban: what can be done and when. */
export function unbanLine(u: Unban, now: number): string {
  switch (u.state) {
    case 'off':
      return 'This channel doesn’t take unban requests.';
    case 'wait':
      return `Available in ${left(u.at, now)}`;
    case 'pending':
      return `Unban request sent ${formatDate(new Date(Date.parse(u.request.createdAt) || now))}. Waiting for a mod.`;
    case 'ready':
    case 'closed':
      return u.last ? pastLine(u.last) : u.state === 'closed' ? 'You can’t ask for an unban here.' : '';
  }
}

function pastLine(r: PastUnban): string {
  switch (r.status) {
    case 'DENIED':
      return r.note ? `Unban request denied: ${r.note}` : 'Unban request denied.';
    case 'APPROVED':
      return 'Unban request approved.';
    case 'CANCELED':
      return 'Unban request canceled.';
    default:
      return '';
  }
}

/** When the next read is due (ms from now), or null: a ban looks again every 3 min, a
 * timeout right after it ends, and IRC's word right after a read that came too soon. */
export function nextRead(lock: Lock | null, input: LockInput): number | null {
  if (input.said && saidCounts(input.said, input.readAt) && input.readAt >= input.said.at) return 5_000;
  if (lock?.kind === 'timeout') return Math.max(1_000, lock.until - input.now + 1_000);
  if (lock?.kind === 'ban') {
    const u = lock.unban;
    if (u.state === 'wait') return Math.min(180_000, Math.max(1_000, u.at - input.now + 1_000));
    return 180_000;
  }
  return null;
}

/** V108-C40b Why spending (points, Bits, power-ups) is off while the box is locked; '' when it isn't. */
export function lockedWhy(lock: Lock | null): string {
  switch (lock?.kind) {
    case 'timeout':
      return 'Not while you’re timed out.';
    case 'ban':
      return 'Not while you’re banned.';
    case 'warn':
      return 'Not until you acknowledge the warning.';
    default:
      return '';
  }
}

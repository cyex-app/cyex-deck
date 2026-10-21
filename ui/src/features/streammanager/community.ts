// V107-R40 Pure helpers for the Community panels (followers, assigned roles), free of
// Svelte so they run under the UI tests.

import type { CmAssigned } from '../../ipc/bindings/CmAssigned';
import type { CmFollower } from '../../ipc/bindings/CmFollower';
import type { CmRole } from '../../ipc/bindings/CmRole';
import { formatDate, formatTime } from '../../lib/format.ts';

export type FollowSort = 'newest' | 'oldest' | 'name';

export const FOLLOW_SORTS: { value: FollowSort; label: string }[] = [
  { value: 'newest', label: 'Newest' },
  { value: 'oldest', label: 'Oldest' },
  { value: 'name', label: 'Name' },
];

function parsed(iso: string | null | undefined): Date | null {
  if (!iso) return null;
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? null : d;
}

/** `2026/10/20 21:05`, local time, 24 hours; empty when Twitch gave none. */
export function stamp(iso: string | null | undefined): string {
  const d = parsed(iso);
  return d ? `${formatDate(d)} ${formatTime(d)}` : '';
}

/** `2026/10/20`, for lists where the time is noise. */
export function day(iso: string | null | undefined): string {
  const d = parsed(iso);
  return d ? formatDate(d) : '';
}

const UNITS: [name: string, secs: number][] = [
  ['year', 31_536_000],
  ['month', 2_592_000],
  ['day', 86_400],
  ['hour', 3_600],
  ['minute', 60],
];

/** "3 days ago", for a hover. */
export function relative(iso: string | null | undefined, now = Date.now()): string {
  const d = parsed(iso);
  if (!d) return '';
  const s = Math.max(0, Math.floor((now - d.getTime()) / 1000));
  for (const [name, size] of UNITS) {
    const n = Math.floor(s / size);
    if (n >= 1) return `${n} ${name}${n === 1 ? '' : 's'} ago`;
  }
  return 'just now';
}

/** A next page added to what is loaded; one already there is kept once. */
export function merged(have: CmFollower[], page: CmFollower[]): CmFollower[] {
  const seen = new Set(have.map((f) => f.id));
  return [...have, ...page.filter((f) => !seen.has(f.id))];
}

/** What shows: the search over name and login, then the order. Date orders are Twitch's
 *  own (`newest` and `oldest` come loaded that way); only the name is sorted here. */
export function shownFollowers(list: CmFollower[], query: string, sort: FollowSort): CmFollower[] {
  const q = query.trim().toLowerCase();
  const found = q ? list.filter((f) => f.login.includes(q) || f.name.toLowerCase().includes(q)) : list;
  return sort === 'name' ? [...found].sort((a, b) => a.name.localeCompare(b.name, undefined, { sensitivity: 'base' })) : found;
}

export const ROLE_ORDER: CmRole[] = ['moderator', 'leadMod', 'editor', 'manager'];

export const ROLE_LABEL: Record<CmRole, string> = {
  moderator: 'Moderator',
  leadMod: 'Lead mod',
  editor: 'Editor',
  manager: 'Business manager',
};

/** The button, and what it says once armed. */
export const ROLE_LEAVE: Record<CmRole, { label: string; confirm: string }> = {
  moderator: { label: 'Unmod', confirm: 'Unmod yourself?' },
  leadMod: { label: 'Step down', confirm: 'Step down?' },
  editor: { label: 'Step down', confirm: 'Leave?' },
  manager: { label: 'Step down', confirm: 'Step down?' },
};

export type RoleFilter = CmRole | 'all';

/** What shows: one role or all, narrowed by search; channels in name order, a channel's
 *  roles in `ROLE_ORDER`. */
export function shownRoles(list: CmAssigned[], role: RoleFilter, query: string): CmAssigned[] {
  const q = query.trim().toLowerCase();
  return list
    .filter((r) => (role === 'all' || r.role === role) && (!q || r.login.includes(q) || r.name.toLowerCase().includes(q)))
    .sort(
      (a, b) =>
        a.name.localeCompare(b.name, undefined, { sensitivity: 'base' }) || ROLE_ORDER.indexOf(a.role) - ROLE_ORDER.indexOf(b.role),
    );
}

/** How many channels hold each role. */
export function roleCounts(list: CmAssigned[]): Record<RoleFilter, number> {
  const out: Record<RoleFilter, number> = { all: list.length, moderator: 0, leadMod: 0, editor: 0, manager: 0 };
  for (const r of list) out[r.role]++;
  return out;
}

/** A role row's key. */
export const roleKey = (r: Pick<CmAssigned, 'channelId' | 'role'>) => `${r.channelId}/${r.role}`;

// V107-R40 Pure helpers for the Community panels (followers, assigned roles), free of
// Svelte so they run under the UI tests.

import type { CmAssigned } from '../../ipc/bindings/CmAssigned';
import type { CmFollower } from '../../ipc/bindings/CmFollower';
import type { ChannelRole } from '../../ipc/bindings/ChannelRole';
import { ago, formatDate, formatTime } from '../../lib/format.ts';

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

/** "3 days ago", for a hover; empty when Twitch gave none (lib/format `ago`). */
export function relative(iso: string | null | undefined, now = Date.now()): string {
  const d = parsed(iso);
  return d ? ago(d.getTime(), now, true) : '';
}

/** What shows: the search over name and login, then the order. Date orders are Twitch's
 *  own (`newest` and `oldest` come loaded that way); only the name is sorted here. */
export function shownFollowers(list: CmFollower[], query: string, sort: FollowSort): CmFollower[] {
  const q = query.trim().toLowerCase();
  const found = q ? list.filter((f) => f.login.includes(q) || f.display.toLowerCase().includes(q)) : list;
  return sort === 'name' ? [...found].sort((a, b) => a.display.localeCompare(b.display, undefined, { sensitivity: 'base' })) : found;
}

/** V107-R42 The roles this list holds (one role type across the app: `ChannelRole`). */
export const ROLE_ORDER: ChannelRole[] = ['mod', 'leadMod', 'editor', 'agent'];

export const ROLE_LABEL: Record<ChannelRole, string> = {
  mod: 'Moderator',
  leadMod: 'Lead mod',
  vip: 'VIP',
  editor: 'Editor',
  artist: 'Artist',
  agent: 'Business manager',
};

/** The button, and what it says once armed. */
const STEP_DOWN = { label: 'Step down', confirm: 'Step down?' };
export const ROLE_LEAVE: Record<ChannelRole, { label: string; confirm: string }> = {
  mod: { label: 'Unmod', confirm: 'Unmod yourself?' },
  leadMod: STEP_DOWN,
  vip: STEP_DOWN,
  editor: STEP_DOWN,
  artist: STEP_DOWN,
  agent: STEP_DOWN,
};

export type RoleFilter = ChannelRole | 'all';

/** What shows: one role or all, narrowed by search; channels in name order, a channel's
 *  roles in `ROLE_ORDER`. */
export function shownRoles(list: CmAssigned[], role: RoleFilter, query: string): CmAssigned[] {
  const q = query.trim().toLowerCase();
  return list
    .filter((r) => (role === 'all' || r.role === role) && (!q || r.login.includes(q) || r.display.toLowerCase().includes(q)))
    .sort(
      (a, b) =>
        a.display.localeCompare(b.display, undefined, { sensitivity: 'base' }) || ROLE_ORDER.indexOf(a.role) - ROLE_ORDER.indexOf(b.role),
    );
}

/** How many channels hold each role. */
export function roleCounts(list: CmAssigned[]): Record<RoleFilter, number> {
  const out: Record<RoleFilter, number> = { all: list.length, leadMod: 0, mod: 0, vip: 0, editor: 0, artist: 0, agent: 0 };
  for (const r of list) out[r.role]++;
  return out;
}

/** A role row's key. */
export const roleKey = (r: Pick<CmAssigned, 'id' | 'role'>) => `${r.id}/${r.role}`;

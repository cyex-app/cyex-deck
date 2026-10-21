// V107-R36 Pure helpers for the Roles Manager panel: names, the filter, the sort. Free of
// Svelte so they run under the UI tests.

import type { ChannelRole } from '../../ipc/bindings/ChannelRole';
import type { RoleMember } from '../../ipc/bindings/RoleMember';
import type { VipSlots } from '../../ipc/bindings/VipSlots';

/** Twitch's order. */
export const ROLES: ChannelRole[] = ['leadMod', 'mod', 'vip', 'editor', 'artist', 'agent'];

export const ROLE_NAME: Record<ChannelRole, string> = {
  leadMod: 'Lead Moderator',
  mod: 'Moderator',
  vip: 'VIP',
  editor: 'Editor',
  artist: 'Artist',
  agent: 'Agent',
};

export const ROLE_HINT: Record<ChannelRole, string> = {
  leadMod: 'Manages your mods, and can mod and VIP people.',
  mod: 'Moderates your chat.',
  vip: 'A badge, and no slow mode or chat limits.',
  editor: 'Edits your stream info, videos and clips.',
  artist: 'Makes emotes and badges for you.',
  agent: 'Handles business for you.',
};

export type RoleFilter = ChannelRole | 'all';
export type RoleSort = 'name' | 'nameDesc' | 'newest' | 'oldest';

export const SORTS: { value: RoleSort; label: string }[] = [
  { value: 'newest', label: 'Newest' },
  { value: 'oldest', label: 'Oldest' },
  { value: 'name', label: 'A to Z' },
  { value: 'nameDesc', label: 'Z to A' },
];

const byName = (a: RoleMember, b: RoleMember) => a.login.localeCompare(b.login);
const time = (m: RoleMember) => (m.modified ? Date.parse(m.modified) || 0 : 0);

/** Newest (`dir` -1) or oldest (`dir` 1) first; rows with no date go last either way. */
function byTime(a: RoleMember, b: RoleMember, dir: 1 | -1): number {
  const ta = time(a);
  const tb = time(b);
  if (!ta || !tb) return (ta ? -1 : 0) + (tb ? 1 : 0) || byName(a, b);
  return (ta - tb) * dir || byName(a, b);
}

/** What shows: one role or all, the search over name and login, then the order. */
export function shownMembers(list: RoleMember[], role: RoleFilter, query: string, sort: RoleSort): RoleMember[] {
  const q = cleanLogin(query);
  const found = list.filter(
    (m) => (role === 'all' || m.roles.includes(role)) && (!q || m.login.includes(q) || m.display.toLowerCase().includes(q)),
  );
  const cmp: Record<RoleSort, (a: RoleMember, b: RoleMember) => number> = {
    name: byName,
    nameDesc: (a, b) => byName(b, a),
    newest: (a, b) => byTime(a, b, -1),
    oldest: (a, b) => byTime(a, b, 1),
  };
  return [...found].sort(cmp[sort]);
}

/** How many hold each role. */
export function roleCounts(list: RoleMember[]): Record<RoleFilter, number> {
  const out = { all: list.length } as Record<RoleFilter, number>;
  for (const r of ROLES) out[r] = 0;
  for (const m of list) for (const r of m.roles) out[r]++;
  return out;
}

/** V107-R42 Why giving `role` to someone holding `holds` is off: Rust's VIP slot line
 *  (`vips.why`), only for a VIP grant. The order, swaps and lines are Rust's too. */
export function offFor(role: ChannelRole, holds: ChannelRole[], vips: VipSlots | null): string | null {
  return role === 'vip' && !holds.includes('vip') ? (vips?.why ?? null) : null;
}

/** A login as typed: no @, no spaces, lower case. */
export function cleanLogin(s: string): string {
  return s.trim().replace(/^@/, '').toLowerCase();
}

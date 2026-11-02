// V108-C26 Pure helpers for the roles pop-up (/mods, /vips, /roles): which groups show, the
// search, the order. Free of Svelte so they run under the UI tests.

import type { ChannelRole } from '../../ipc/bindings/ChannelRole';
import type { RoleGroup } from '../../ipc/bindings/RoleGroup';
import type { RoleHolder } from '../../ipc/bindings/RoleHolder';
import { byNumber, byText, NAME, type Sort, sortRows, TIME } from '../../lib/sort.ts';

/** Group titles, plural: they head a list with a count. */
export const GROUP_NAME: Record<ChannelRole, string> = {
  leadMod: 'Lead mods',
  mod: 'Mods',
  vip: 'VIPs',
  editor: 'Editors',
  artist: 'Artists',
  agent: 'Agents',
};

export const SORTS = [TIME, NAME];
export type ListSort = Sort<'time' | 'name'>;
export const LIST_SORT: ListSort = { key: 'time', desc: true };

// V108-F03 By the name shown (display, any case, locale-aware); the login breaks a tie.
const byName = (a: RoleHolder, b: RoleHolder) => byText(a.display, b.display) || byText(a.login, b.login);
/** People with no date go last either way. */
const byTime = byNumber((p: RoleHolder) => (p.grantedAt ? Date.parse(p.grantedAt) || 0 : 0));

/** The title: one role's name, or all of them. */
export function listTitle(role: ChannelRole | null): string {
  return role ? GROUP_NAME[role] : 'Roles';
}

/** The groups that show: just `role` when one is asked for (even empty, so it can say so),
 *  else every group with someone in it. Each group is searched over name and login, then
 *  ordered; a group the search empties drops out. */
export function shownGroups(groups: RoleGroup[], role: ChannelRole | null, query: string, sort: ListSort): RoleGroup[] {
  const q = query.trim().replace(/^@/, '').toLowerCase();
  const out: RoleGroup[] = [];
  for (const g of groups) {
    if (role ? g.role !== role : g.people.length === 0) continue;
    const found = q ? g.people.filter((p) => p.login.includes(q) || p.display.toLowerCase().includes(q)) : g.people;
    if (role || found.length) out.push({ role: g.role, people: sortRows(found, sort, { time: byTime, name: byName }, byName) });
  }
  return out;
}

/** How many people the groups hold, for the header. */
export function total(groups: RoleGroup[]): number {
  return groups.reduce((n, g) => n + g.people.length, 0);
}

/** V108-C27 The list's own window: `roles/<channel>/<role or all>/<account or nothing>`. */
export function rolesRoute(channel: string, role: ChannelRole | null, account: string | null): string {
  return `roles/${channel}/${role ?? 'all'}/${account ?? ''}`;
}

const ROLE_KEYS = new Set<string>(Object.keys(GROUP_NAME));

/** V108-C27 The route back as its parts; an unknown role reads as all of them. */
export function parseRolesRoute(route: string): { channel: string; role: ChannelRole | null; account: string | null } {
  const [, channel = '', role = '', account = ''] = route.split('/');
  return { channel, role: ROLE_KEYS.has(role) ? (role as ChannelRole) : null, account: account || null };
}

/** V108-C27 What to say about roles Twitch kept back here (mods, signed out); null when
 *  every role asked for came. */
export function unreadNote(unread: ChannelRole[], role: ChannelRole | null, signedIn: boolean): string | null {
  const missing = role ? unread.filter((r) => r === role) : unread;
  if (!missing.length) return null;
  const names = missing.map((r) => GROUP_NAME[r].toLowerCase()).join(' and ');
  return signedIn ? `Twitch didn’t share the ${names}.` : `Sign in to see the ${names}.`;
}

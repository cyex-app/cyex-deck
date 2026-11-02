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

const byName = (a: RoleHolder, b: RoleHolder) => byText(a.login, b.login);
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

import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { RoleGroup } from '../../ipc/bindings/RoleGroup';
import type { RoleHolder } from '../../ipc/bindings/RoleHolder';
import { LIST_SORT, listTitle, parseRolesRoute, rolesRoute, shownGroups, total, unreadNote } from './roleList.ts';

const p = (login: string, grantedAt: string | null = null): RoleHolder => ({ id: login, login, display: login.toUpperCase(), avatar: '', grantedAt });
const groups: RoleGroup[] = [
  { role: 'leadMod', people: [] },
  { role: 'mod', people: [p('old', '2020-01-01T00:00:00Z'), p('none'), p('new', '2026-05-01T00:00:00Z')] },
  { role: 'vip', people: [p('zed', '2024-01-01T00:00:00Z'), p('amy', '2024-01-01T00:00:00Z')] },
];
const logins = (g: RoleGroup) => g.people.map((x) => x.login);

test('all roles: empty groups drop out, newest first, undated last, ties by name', () => {
  const s = shownGroups(groups, null, '', LIST_SORT);
  assert.deepEqual(s.map((g) => g.role), ['mod', 'vip']);
  assert.deepEqual(logins(s[0]), ['new', 'old', 'none']);
  assert.deepEqual(logins(s[1]), ['amy', 'zed']);
});

test('one role stays even when empty', () => {
  const s = shownGroups(groups, 'leadMod', '', LIST_SORT);
  assert.deepEqual(s, [{ role: 'leadMod', people: [] }]);
});

test('sorting A-Z and the search over login and name', () => {
  const az = shownGroups(groups, 'mod', '', { key: 'name', desc: false });
  assert.deepEqual(logins(az[0]), ['new', 'none', 'old']);
  assert.deepEqual(logins(shownGroups(groups, 'mod', ' @OL', LIST_SORT)[0]), ['old']);
  assert.deepEqual(shownGroups(groups, null, 'zed', LIST_SORT).map((g) => g.role), ['vip']);
  assert.deepEqual(shownGroups(groups, null, 'nobody', LIST_SORT), []);
});

test('titles and totals', () => {
  assert.equal(listTitle('vip'), 'VIPs');
  assert.equal(listTitle(null), 'Roles');
  assert.equal(total(groups), 5);
});

test('V108-C27 the window route round-trips, an odd role is all', () => {
  assert.equal(rolesRoute('xqc', 'mod', '123'), 'roles/xqc/mod/123');
  assert.equal(rolesRoute('xqc', null, null), 'roles/xqc/all/');
  assert.deepEqual(parseRolesRoute('roles/xqc/mod/123'), { channel: 'xqc', role: 'mod', account: '123' });
  assert.deepEqual(parseRolesRoute('roles/xqc/all/'), { channel: 'xqc', role: null, account: null });
  assert.deepEqual(parseRolesRoute('roles/xqc/nope'), { channel: 'xqc', role: null, account: null });
});

test('V108-C27 a note for the roles Twitch kept back', () => {
  assert.equal(unreadNote([], null, false), null);
  assert.equal(unreadNote(['mod'], null, false), 'Sign in to see the mods.');
  assert.equal(unreadNote(['mod'], 'mod', true), 'Twitch didn’t share the mods.');
  assert.equal(unreadNote(['mod'], 'vip', false), null, 'only what was asked for');
});

import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { RoleMember } from '../../ipc/bindings/RoleMember';
import { cleanLogin, offFor, roleCounts, shownMembers } from './rolesManager.ts';

const m = (login: string, roles: RoleMember['roles'], modified: string | null = null): RoleMember => ({
  id: login,
  login,
  display: login.toUpperCase(),
  avatar: '',
  roles,
  modified,
});

const list = [m('bob', ['mod'], '2024-01-01T00:00:00Z'), m('ann', ['vip'], '2025-01-01T00:00:00Z'), m('cat', ['editor', 'vip'])];
const logins = (l: RoleMember[]) => l.map((x) => x.login);

test('v107-r36 filter, search and every sort', () => {
  assert.deepEqual(logins(shownMembers(list, 'all', '', 'name')), ['ann', 'bob', 'cat']);
  assert.deepEqual(logins(shownMembers(list, 'all', '', 'nameDesc')), ['cat', 'bob', 'ann']);
  assert.deepEqual(logins(shownMembers(list, 'all', '', 'newest')), ['ann', 'bob', 'cat']);
  assert.deepEqual(logins(shownMembers(list, 'all', '', 'oldest')), ['bob', 'ann', 'cat'], 'no date last');
  assert.deepEqual(logins(shownMembers(list, 'vip', '', 'name')), ['ann', 'cat']);
  assert.deepEqual(logins(shownMembers(list, 'all', '@BO', 'name')), ['bob']);
  assert.deepEqual(logins(shownMembers(list, 'all', 'CAT', 'name')), ['cat'], 'display name too');
});

test('v107-r42 counts and the VIP slot line from Rust', () => {
  const c = roleCounts(list);
  assert.equal(c.all, 3);
  assert.equal(c.vip, 2);
  assert.equal(c.agent, 0);
  const full = { used: 10, allowed: 10, state: 'full' as const, why: 'Every VIP slot is taken. Remove a VIP first.', label: 'VIPs 10/10' };
  assert.equal(offFor('vip', [], full), full.why);
  assert.equal(offFor('vip', ['vip'], full), null, 'removing is fine');
  assert.equal(offFor('mod', [], full), null);
  assert.equal(offFor('vip', [], null), null, 'not known: Twitch decides');
  assert.equal(cleanLogin(' @Caressz '), 'caressz');
});

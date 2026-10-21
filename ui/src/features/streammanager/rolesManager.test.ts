import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { RoleMember } from '../../ipc/bindings/RoleMember';
import { addOrder, blocked, cleanLogin, roleCounts, shownMembers, vipCount } from './rolesManager.ts';

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

test('v107-r36 counts, slots and the add order', () => {
  const c = roleCounts(list);
  assert.equal(c.all, 3);
  assert.equal(c.vip, 2);
  assert.equal(c.agent, 0);
  assert.equal(blocked('vip', [], { used: 0, allowed: 0, state: 'locked' }), 'VIPs unlock through Twitch achievements.');
  assert.equal(blocked('vip', [], { used: 10, allowed: 10, state: 'full' }), 'Every VIP slot is taken.');
  assert.equal(blocked('vip', ['vip'], { used: 10, allowed: 10, state: 'full' }), null, 'removing is fine');
  assert.equal(blocked('mod', [], { used: 10, allowed: 10, state: 'full' }), null);
  assert.deepEqual(addOrder(['mod', 'leadMod', 'editor'], []), ['leadMod', 'editor']);
  assert.deepEqual(addOrder(['vip', 'mod'], ['vip']), ['mod']);
  assert.equal(vipCount({ used: 7, allowed: 10, state: 'open' }), 'VIPs 7/10');
  assert.equal(vipCount({ used: 7, allowed: null, state: 'unknown' }), 'VIPs 7');
  assert.equal(vipCount({ used: 0, allowed: 0, state: 'locked' }), 'VIPs locked');
  assert.equal(cleanLogin(' @Caressz '), 'caressz');
});

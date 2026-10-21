import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { CmAssigned } from '../../ipc/bindings/CmAssigned';
import type { CmFollower } from '../../ipc/bindings/CmFollower';
import { day, relative, roleCounts, roleKey, shownFollowers, shownRoles, stamp } from './community.ts';

const f = (id: string, login: string, name = login): CmFollower => ({ id, login, display: name, avatar: '', followedAt: null });
const r = (channelId: string, login: string, role: CmAssigned['role'], name = login): CmAssigned => ({
  id: channelId,
  login,
  display: name,
  avatar: '',
  role,
  since: null,
});

test('V107-R40 dates are yyyy/mm/dd, 24 hours', () => {
  const iso = new Date(2026, 9, 5, 7, 8).toISOString();
  assert.equal(stamp(iso), '2026/10/05 07:08');
  assert.equal(day(iso), '2026/10/05');
  assert.equal(stamp(null), '');
  assert.equal(stamp('nope'), '');
});

test('V107-R40 relative time reads in words', () => {
  const now = Date.UTC(2026, 9, 21, 12, 0, 0);
  const ago = (s: number) => relative(new Date(now - s * 1000).toISOString(), now);
  assert.equal(ago(10), 'just now');
  assert.equal(ago(60), '1 minute ago');
  assert.equal(ago(3 * 3600), '3 hours ago');
  assert.equal(ago(2 * 86_400), '2 days ago');
  assert.equal(ago(70 * 86_400), '2 months ago');
  assert.equal(ago(800 * 86_400), '2 years ago');
  assert.equal(relative(undefined, now), '');
});

test('V107-R40 search matches name or login, name sort ignores case, dates keep Twitch order', () => {
  const list = [f('1', 'zed', 'Zed'), f('2', 'amy', 'amy'), f('3', 'bob', 'Bob')];
  assert.deepEqual(shownFollowers(list, ' BO ', 'newest').map((x) => x.id), ['3']);
  assert.deepEqual(shownFollowers(list, '', 'name').map((x) => x.id), ['2', '3', '1']);
  assert.deepEqual(shownFollowers(list, '', 'oldest').map((x) => x.id), ['1', '2', '3']);
});

test('V107-R40 roles filter, search and order', () => {
  const list = [r('2', 'bob', 'editor'), r('1', 'amy', 'leadMod'), r('1', 'amy', 'mod'), r('3', 'cy', 'mod')];
  assert.deepEqual(shownRoles(list, 'all', '').map(roleKey), ['1/mod', '1/leadMod', '2/editor', '3/mod']);
  assert.deepEqual(shownRoles(list, 'mod', 'c').map(roleKey), ['3/mod']);
  assert.deepEqual(roleCounts(list), { all: 4, leadMod: 1, mod: 2, vip: 0, editor: 1, artist: 0, agent: 0 });
});

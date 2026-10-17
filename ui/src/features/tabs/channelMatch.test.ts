import assert from 'node:assert/strict';
import { test } from 'node:test';

import { liveOrExact, matchChannels, type Candidate } from './channelMatch.ts';

const ch = (id: string, login: string, live = false, name = login): Candidate => ({
  id,
  login,
  name,
  stream: live ? { viewers: 1 } : null,
});
const ids = (list: Candidate[]) => list.map((c) => c.id);

test('an offline exact name beats a page of live hits', () => {
  const exact = [ch('1', 'sery')];
  const search = [1, 2, 3, 4, 5, 6, 7].map((n) => ch(`l${n}`, `sery${n}`, true)).concat(ch('1', 'sery'));
  const out = matchChannels(exact, search, 'sery', 6);
  assert.equal(out.length, 6);
  assert.equal(out[0].id, '1');
  assert.equal(new Set(ids(out)).size, 6);
});

test('an offline exact name stays in even when the lookup missed it', () => {
  const search = [1, 2, 3, 4, 5, 6].map((n) => ch(`l${n}`, `xsery${n}`, true)).concat(ch('1', 'Sery'));
  assert.equal(matchChannels([], search, 'sery', 6)[0].id, '1');
});

test('a link or id match is pinned on top', () => {
  const out = matchChannels([ch('71092938', 'xqc')], [ch('2', '71092938a', true)], '71092938', 6);
  assert.deepEqual(ids(out), ['71092938', '2']);
});

test('better match first, ties go live first, then the order they came in', () => {
  const search = [ch('a', 'foobar'), ch('b', 'foobaz', true), ch('c', 'my_foo', true), ch('d', 'foo_x')];
  // starts: a, b, d (b live first), then the word match c.
  assert.deepEqual(ids(matchChannels([], search, 'foo', 6)), ['b', 'a', 'd', 'c']);
});

test('the display name counts too, and nothing that matches is lost', () => {
  const search = [ch('a', 'zzz', false), ch('b', 'qq', false, 'Foo')];
  assert.deepEqual(ids(matchChannels([], search, 'foo', 6)), ['b', 'a']);
});

test('duplicates are dropped by id', () => {
  const out = matchChannels([ch('1', 'a')], [ch('1', 'a'), ch('2', 'ab'), ch('2', 'ab')], 'a', 6);
  assert.deepEqual(ids(out), ['1', '2']);
});

test('the switcher keeps live hits and the offline exact name', () => {
  const items = [ch('1', 'sery'), ch('2', 'sery2'), ch('3', 'sery3', true), ch('4', 'x', false, 'Sery')];
  assert.deepEqual(ids(liveOrExact(items, ' SERY ')), ['1', '3', '4']);
});

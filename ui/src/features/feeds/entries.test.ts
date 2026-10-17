import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { FeedEntry } from '../../ipc/bindings/FeedEntry';
import { apply, keyOf, tsOf } from './entries.ts';

function live(key: string, on = true, ts = 1): FeedEntry {
  return { type: 'live', key, ts, channelId: '1', login: 'a', name: 'A', avatar: null, live: on, title: null, game: null };
}
const keys = (l: FeedEntry[]) => l.map(keyOf);

test('add puts the newest first', () => {
  const l = apply([], [{ type: 'add', entry: live('a') }, { type: 'add', entry: live('b') }], 10);
  assert.deepEqual(keys(l), ['b', 'a']);
});

test('a duplicate add replaces in place', () => {
  const start = apply([], [{ type: 'add', entry: live('a') }, { type: 'add', entry: live('b') }], 10);
  const l = apply(start, [{ type: 'add', entry: live('a', false) }], 10);
  assert.deepEqual(keys(l), ['b', 'a']);
  assert.equal((l[1] as { live: boolean }).live, false);
});

test('update replaces by key; an unknown key is ignored', () => {
  const start = [live('a')];
  const l = apply(start, [{ type: 'update', entry: live('a', false) }, { type: 'update', entry: live('zz') }], 10);
  assert.deepEqual(keys(l), ['a']);
  assert.equal((l[0] as { live: boolean }).live, false);
});

test('remove and clear', () => {
  const start = [live('a'), live('b')];
  assert.deepEqual(keys(apply(start, [{ type: 'remove', key: 'a' }], 10)), ['b']);
  assert.deepEqual(apply(start, [{ type: 'clear' }], 10), []);
});

test('cap cuts the oldest', () => {
  const evs = ['a', 'b', 'c', 'd'].map((k) => ({ type: 'add' as const, entry: live(k) }));
  assert.deepEqual(keys(apply([], evs, 3)), ['d', 'c', 'b']);
});

test('tsOf reads the time', () => {
  assert.equal(tsOf(live('a', true, 42)), 42);
});

import assert from 'node:assert/strict';
import { test } from 'node:test';

import { CappedMap, dropOldest, dropStale, keepOnly, setCapped, useCapped } from './capped.ts';

test('setCapped keeps the newest and drops the oldest past the cap', () => {
  const m = new Map<string, number>();
  for (const [i, k] of ['a', 'b', 'c', 'd'].entries()) setCapped(m, k, i, 3);
  assert.deepEqual([...m.keys()], ['b', 'c', 'd']);
  setCapped(m, 'b', 9, 3);
  assert.deepEqual([...m.keys()], ['c', 'd', 'b'], 'a set again is the newest');
  assert.equal(m.get('b'), 9);
});

test('useCapped marks a key as used, so it outlives older ones', () => {
  const m = new Map([['a', 1], ['b', 2], ['c', 3]]);
  assert.equal(useCapped(m, 'a'), 1);
  assert.equal(useCapped(m, 'x'), undefined);
  assert.equal(m.has('x'), false, 'a miss adds nothing');
  setCapped(m, 'd', 4, 3);
  assert.deepEqual([...m.keys()], ['c', 'a', 'd']);
});

test('dropOldest, dropStale and keepOnly', () => {
  const m = new Map([['a', 1], ['b', 2], ['c', 3]]);
  dropOldest(m, 5);
  assert.equal(m.size, 3);
  dropOldest(m, 0);
  assert.equal(m.size, 0);
  const n = new Map([['a', 1], ['b', 20], ['c', 3]]);
  dropStale(n, (v) => v > 10);
  assert.deepEqual([...n.keys()], ['a', 'c']);
  keepOnly(n, (k) => k === 'c');
  assert.deepEqual([...n.keys()], ['c']);
});

test('CappedMap drops the oldest set past its cap', () => {
  const m = new CappedMap<string, number>(2);
  m.set('a', 1).set('b', 2).set('a', 3).set('c', 4);
  assert.deepEqual([...m.entries()], [['a', 3], ['c', 4]]);
});

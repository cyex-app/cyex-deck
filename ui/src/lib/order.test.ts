import assert from 'node:assert/strict';
import { test } from 'node:test';

import { moveKey, moveTo } from './order.ts';

const abcd = ['a', 'b', 'c', 'd'];

test('moves a row before or after the one under the cursor', () => {
  assert.deepEqual(moveTo(abcd, 0, 2, false), ['b', 'a', 'c', 'd']);
  assert.deepEqual(moveTo(abcd, 0, 2, true), ['b', 'c', 'a', 'd']);
  assert.deepEqual(moveTo(abcd, 3, 0, false), ['d', 'a', 'b', 'c']);
  assert.deepEqual(moveTo(abcd, 1, 3, true), ['a', 'c', 'd', 'b']);
});

test('dropping next to itself changes nothing', () => {
  assert.deepEqual(moveTo(abcd, 2, 2, false), abcd);
  assert.deepEqual(moveTo(abcd, 2, 1, true), abcd);
});

test('does not touch the input', () => {
  const list = [...abcd];
  moveTo(list, 0, 3, true);
  assert.deepEqual(list, abcd);
});

test('moves by key', () => {
  const key = (s: string) => s;
  assert.deepEqual(moveKey(abcd, key, 'a', 'c', true), ['b', 'c', 'a', 'd']);
  assert.deepEqual(moveKey(abcd, key, 'd', 'a', false), ['d', 'a', 'b', 'c']);
  assert.equal(moveKey(abcd, key, 'b', 'b', false), null);
  assert.equal(moveKey(abcd, key, 'b', 'a', true), null);
  assert.equal(moveKey(abcd, key, 'x', 'a', false), null);
});

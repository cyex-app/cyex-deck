import assert from 'node:assert/strict';
import { test } from 'node:test';

import { moveTo } from './order.ts';

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

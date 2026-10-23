import assert from 'node:assert/strict';
import { test } from 'node:test';

import { blocked, NOT_ENOUGH, short } from './rewards.ts';

test('V107-H11b a reward over the balance is short', () => {
  assert.equal(short(1000, 999), true);
  assert.equal(short(1000, 1000), false);
  assert.equal(short(0, 0), false);
  assert.equal(short(1, 0), true);
});

test('V107-H11b an unknown balance marks nothing', () => {
  assert.equal(short(1000, null), false);
  assert.equal(short(1000, undefined), false);
  assert.equal(blocked(1000, null), null);
});

test('V107-H11b redeeming a short reward says so', () => {
  assert.equal(blocked(500, 100), `${NOT_ENOUGH}.`);
  assert.equal(blocked(100, 500), null);
});

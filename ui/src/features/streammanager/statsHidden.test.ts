import assert from 'node:assert/strict';
import { test } from 'node:test';

import { isHidden, toggleStat } from './statsHidden.ts';

test('V108-C38 a stat hides, then shows again', () => {
  const a = toggleStat([], 'subs');
  assert.deepEqual(a, ['subs']);
  assert.ok(isHidden(a, 'subs'));
  assert.deepEqual(toggleStat(a, 'subs'), []);
});

test('V108-C38 toggling keeps the others and does not mutate', () => {
  const start = ['viewers', 'subs'];
  assert.deepEqual(toggleStat(start, 'viewers'), ['subs']);
  assert.deepEqual(toggleStat(start, 'followers'), ['viewers', 'subs', 'followers']);
  assert.deepEqual(start, ['viewers', 'subs']);
});

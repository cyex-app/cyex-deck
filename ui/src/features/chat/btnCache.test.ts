import assert from 'node:assert/strict';
import { test } from 'node:test';
import { btnMemo, remember } from './btnCache.ts';

test('remembers per channel, case-insensitive, merging patches (V107-My)', () => {
  assert.deepEqual(btnMemo('Nobody'), {});
  remember('Rvbdev', { bits: true });
  remember('rvbdev', { pointsAway: false, pointsIcon: 'x' });
  assert.deepEqual(btnMemo('RVBDEV'), { bits: true, pointsAway: false, pointsIcon: 'x' });
});

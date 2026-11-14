import assert from 'node:assert/strict';
import { test } from 'node:test';

import { foldPresence } from './presenceFold.ts';

const names = (n: number) => Array.from({ length: n }, (_, i) => `u${i}`);

test('V111-07 off: every name shows, nothing to toggle', () => {
  assert.deepEqual(foldPresence(names(40), false, false), { shown: names(40), hidden: 0, toggle: null });
});

test('V111-07 five or fewer names are never folded', () => {
  assert.deepEqual(foldPresence(names(5), true, false), { shown: names(5), hidden: 0, toggle: null });
});

test('V111-07 six or more fold to five with Show more', () => {
  const r = foldPresence(names(6), true, false);
  assert.deepEqual([r.shown.length, r.hidden, r.toggle], [5, 1, 'Show more']);
  const big = foldPresence(names(300), true, false);
  assert.deepEqual([big.shown.length, big.hidden], [5, 295]);
});

test('V111-07 open shows all with Show less', () => {
  assert.deepEqual(foldPresence(names(12), true, true), { shown: names(12), hidden: 0, toggle: 'Show less' });
});

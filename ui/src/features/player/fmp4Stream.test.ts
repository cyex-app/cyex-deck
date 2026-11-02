import assert from 'node:assert/strict';
import { test } from 'node:test';

import { firstPiece, wrapPush } from './fmp4Stream.ts';

test('V108-LL7 only the first piece of an fMP4 segment keeps the timestamp check', () => {
  const firsts = new WeakMap<object, { stats: object; id: number }>();
  const frag = { initSegment: {}, stats: {} };
  assert.equal(firstPiece(firsts, frag, { id: 1 }), true);
  assert.equal(firstPiece(firsts, frag, { id: 2 }), false);
  assert.equal(firstPiece(firsts, frag, { id: 5 }), false);
  // A retry has new stats and starts over.
  frag.stats = {};
  assert.equal(firstPiece(firsts, frag, { id: 6 }), true);
  assert.equal(firstPiece(firsts, frag, { id: 7 }), false);
});

test('V108-LL7 TS segments (no init) are left alone', () => {
  const firsts = new WeakMap<object, { stats: object; id: number }>();
  const frag = { stats: {} };
  assert.equal(firstPiece(firsts, frag, { id: 1 }), true);
  assert.equal(firstPiece(firsts, frag, { id: 2 }), true);
});

test('V108-LL7 the wrap clears accurateTimeOffset on later pieces only', () => {
  const seen: unknown[] = [];
  const push = wrapPush((...a) => seen.push(a[7]));
  const frag = { initSegment: {}, stats: {} };
  const args = (id: number, accurate: boolean) => [null, null, '', '', frag, null, 0, accurate, { id }];
  push(...args(1, true));
  push(...args(2, true));
  push(...args(3, false));
  assert.deepEqual(seen, [true, false, false]);
});


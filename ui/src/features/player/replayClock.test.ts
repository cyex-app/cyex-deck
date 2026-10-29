import assert from 'node:assert/strict';
import { test } from 'node:test';

import { dateAt, nextAfter, type Piece, posAt } from './replayClock.ts';

const T0 = Date.parse('2026-10-29T12:00:00Z');

/** 2 s segments from playlist second 0; the fourth comes after a 60 s hole. */
const pieces: Piece[] = [
  { sn: 10, start: 0, duration: 2, programDateTime: T0 },
  { sn: 11, start: 2, duration: 2, programDateTime: T0 + 2000 },
  { sn: 12, start: 4, duration: 2, programDateTime: T0 + 4000 },
  { sn: 13, start: 6, duration: 2, programDateTime: T0 + 66_000 },
];

test('date at a playlist position', () => {
  assert.equal(dateAt(pieces, 0), T0);
  assert.equal(dateAt(pieces, 3.5), T0 + 3500);
  assert.equal(dateAt(pieces, 7), T0 + 67_000);
  assert.equal(dateAt(pieces, 99), T0 + 68_000, 'past the end: the last segment’s end');
  assert.equal(dateAt([], 1), null);
  assert.equal(dateAt([{ sn: 1, start: 0, duration: 2, programDateTime: null }], 1), null);
});

test('playlist position of a date', () => {
  assert.equal(posAt(pieces, T0 + 1000), 1);
  assert.equal(posAt(pieces, T0 + 5000), 5);
  assert.equal(posAt(pieces, T0 + 30_000), 6, 'in the hole: the next piece');
  assert.equal(posAt(pieces, T0 - 9000), 0);
  assert.equal(posAt(pieces, T0 + 999_000), 6, 'past the end: the last segment');
  assert.equal(posAt([], T0), null);
});

test('init segments and missing dates are skipped', () => {
  const init: Piece = { sn: 'initSegment', start: 0, duration: 0, programDateTime: T0 - 1 };
  assert.equal(posAt([init, ...pieces], T0 + 1000), 1);
  assert.equal(dateAt([init, ...pieces], 0), T0);
});

test('the next segment after a missing one', () => {
  const origin = T0 - 600_000;
  assert.equal(nextAfter(pieces, 11, origin), 604);
  assert.equal(nextAfter(pieces, 12, origin), 666);
  assert.equal(nextAfter(pieces, 13, origin), null);
});

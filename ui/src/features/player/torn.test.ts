import assert from 'node:assert/strict';
import { test } from 'node:test';

import { isTorn, TORN_LEAD_S, TORN_S, tornDue, tornStart } from './torn.ts';

test('h3 a fragment parsed where the playlist puts it is fine', () => {
  assert.equal(isTorn(480, 480.02), false);
  assert.equal(isTorn(480, 480 - TORN_S), false);
});

test('h3 a fragment parsed far off its spot is torn (a clock reset without a mark)', () => {
  assert.equal(isTorn(494, 494 - 598.5), true);
  assert.equal(isTorn(494, 494 + TORN_S + 0.1), true);
});

test('h3 nothing planned or no time: not torn', () => {
  assert.equal(isTorn(undefined, 0), false);
  assert.equal(isTorn(10, Number.NaN), false);
});

test('h3 the switch waits until the play head reaches the torn fragment', () => {
  assert.equal(tornDue(null, 500), false);
  assert.equal(tornDue(494, 490), false);
  assert.equal(tornDue(494, 494 - TORN_LEAD_S), true);
  assert.equal(tornDue(494, 495), true);
});

test('h3 the new instance starts at the torn fragment only when live falls before it', () => {
  const frags = [
    { sn: 10, start: 20 },
    { sn: 11, start: 22 },
    { sn: 12, start: 24 },
  ];
  assert.equal(tornStart(frags, 12, 22.5), 24);
  assert.equal(tornStart(frags, 11, 22.5), null);
  assert.equal(tornStart(frags, 13, 20), null);
  assert.equal(tornStart(frags, null, 20), null);
  assert.equal(tornStart(frags, 12, null), null);
});

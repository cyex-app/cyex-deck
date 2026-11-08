import assert from 'node:assert/strict';
import { test } from 'node:test';

import { centreShift, HOLD_MS, holding, SETTLE_MS } from './jumpHold.ts';

test('a centred line needs no shift (V110-F43)', () => {
  assert.equal(centreShift({ top: 100, height: 400 }, { top: 290, height: 20 }), 0);
});

test('a line that grew down below the middle scrolls down, one above scrolls up', () => {
  assert.equal(centreShift({ top: 100, height: 400 }, { top: 450, height: 20 }), 160);
  assert.equal(centreShift({ top: 100, height: 400 }, { top: 20, height: 20 }), -270);
});

test('the pill strip shrinks the view: the middle moves up with it', () => {
  // The list lost 40 px at the bottom; the line that was centred is now 20 px low.
  assert.equal(centreShift({ top: 100, height: 360 }, { top: 290, height: 20 }), 20);
});

test('the hold lets go once the line stays put, and after HOLD_MS at most', () => {
  assert.ok(holding(100, 0, 0));
  assert.ok(!holding(SETTLE_MS, 0, 0));
  assert.ok(holding(1500, 0, 1400));
  assert.ok(!holding(HOLD_MS, 0, HOLD_MS - 1));
});

import assert from 'node:assert/strict';
import { test } from 'node:test';

import { inTabOrder, stopAround, wrapTab } from './focusScope.ts';

test('Tab wraps at both ends and starts at an end from outside', () => {
  assert.equal(wrapTab(2, 3, false), 0);
  assert.equal(wrapTab(0, 3, true), 2);
  assert.equal(wrapTab(1, 3, false), 2);
  assert.equal(wrapTab(-1, 3, false), 0);
  assert.equal(wrapTab(-1, 3, true), 2);
  assert.equal(wrapTab(0, 0, false), -1);
});

test('a roving tabindex of -1 is out of the Tab order', () => {
  assert.equal(inTabOrder('-1'), false);
  assert.equal(inTabOrder('0'), true);
  assert.equal(inTabOrder(null), true);
});

test('V110-F70 Tab from a non-stop goes to the next or previous stop in document order', () => {
  // Stops 0..3; the focused element sits between 1 and 2.
  const after = [false, false, true, true];
  assert.equal(stopAround(after, false), 2);
  assert.equal(stopAround(after, true), 1);
  // Past the last stop: wraps to the first (forward), previous is the last (back).
  assert.equal(stopAround([false, false], false), 0);
  assert.equal(stopAround([false, false], true), 1);
  // Before the first stop.
  assert.equal(stopAround([true, true], false), 0);
  assert.equal(stopAround([true, true], true), 1);
  assert.equal(stopAround([], false), -1);
});

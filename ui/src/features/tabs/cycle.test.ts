import assert from 'node:assert/strict';
import { test } from 'node:test';

import { stepTab, WheelSteps } from './cycle.ts';

const ids = ['a', 'b', 'c'];

test('without wrap the ends stop; the wheel and Ctrl+Tab both wrap (V102-04, V107-H15)', () => {
  assert.equal(stepTab(ids, 'a', 1, false), 'b');
  assert.equal(stepTab(ids, 'b', -1, false), 'a');
  assert.equal(stepTab(ids, 'c', 1, false), null, 'no wrap past the last');
  assert.equal(stepTab(ids, 'a', -1, false), null, 'no wrap before the first');
  assert.equal(stepTab(ids, 'c', 1, true), 'a');
  assert.equal(stepTab(ids, 'a', -1, true), 'c');
  // From home: forward is the first tab, back the last.
  assert.equal(stepTab(ids, null, 1, false), 'a');
  assert.equal(stepTab(ids, null, -1, false), 'c');
  assert.equal(stepTab([], null, 1, false), null);
});

test('a notch is one tab, small deltas add up', () => {
  const w = new WheelSteps();
  assert.equal(w.step(100, false, 0), 1);
  assert.equal(w.step(-120, false, 10), -1);
  assert.equal(w.step(3, true, 20), 1, 'line mode');
  // Touchpad: 30 + 30 + 30 is not yet a tab, 30 more is.
  const t = new WheelSteps();
  assert.equal(t.step(30, false, 0), 0);
  assert.equal(t.step(30, false, 16), 0);
  assert.equal(t.step(30, false, 32), 0);
  assert.equal(t.step(30, false, 48), 1);
  // A pause starts over.
  assert.equal(t.step(40, false, 60), 0);
  assert.equal(t.step(40, false, 1000), 0, 'the pause dropped the first 40');
  assert.equal(t.step(40, false, 1010), 0);
  assert.equal(t.step(40, false, 1020), 1);
});

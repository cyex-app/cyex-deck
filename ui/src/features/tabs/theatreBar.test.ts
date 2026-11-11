import assert from 'node:assert/strict';
import { test } from 'node:test';

import { EDGE_PX, HIDDEN, hideIn, HIDE_AFTER_MS, revealStep } from './theatreBar.ts';

const away = { y: 300, overBar: false, focusIn: false };

test('V110-B7 the top 4 px, the bar or focus inside show it at once', () => {
  assert.deepEqual(revealStep(HIDDEN, { ...away, y: EDGE_PX, now: 0 }), { shown: true, since: null });
  assert.deepEqual(revealStep(HIDDEN, { ...away, y: 0, now: 0 }), { shown: true, since: null });
  assert.deepEqual(revealStep(HIDDEN, { ...away, overBar: true, now: 0 }), { shown: true, since: null });
  assert.deepEqual(revealStep(HIDDEN, { ...away, focusIn: true, now: 0 }), { shown: true, since: null });
});

test('V110-B7 below the edge it stays hidden', () => {
  assert.deepEqual(revealStep(HIDDEN, { ...away, y: EDGE_PX + 1, now: 0 }), HIDDEN);
  assert.deepEqual(revealStep(HIDDEN, { y: null, overBar: false, focusIn: false, now: 0 }), HIDDEN);
});

test('V110-B7 it hides 600 ms after the pointer left, not before', () => {
  const shown = { shown: true, since: null };
  const left = revealStep(shown, { ...away, now: 1000 });
  assert.deepEqual(left, { shown: true, since: 1000 });
  assert.equal(hideIn(left, 1000), HIDE_AFTER_MS);
  assert.equal(hideIn(left, 1400), 200);
  assert.deepEqual(revealStep(left, { ...away, now: 1599 }), { shown: true, since: 1000 });
  assert.deepEqual(revealStep(left, { ...away, now: 1600 }), HIDDEN);
});

test('V110-B7 coming back before the 600 ms keeps it and restarts the wait', () => {
  const left = { shown: true, since: 1000 };
  const back = revealStep(left, { ...away, overBar: true, now: 1300 });
  assert.deepEqual(back, { shown: true, since: null });
  assert.equal(hideIn(back, 1300), null);
  assert.deepEqual(revealStep(back, { ...away, now: 2000 }), { shown: true, since: 2000 });
});

test('V110-B7 focus staying in the bar keeps it', () => {
  assert.deepEqual(revealStep({ shown: true, since: 1000 }, { ...away, focusIn: true, now: 5000 }), { shown: true, since: null });
});

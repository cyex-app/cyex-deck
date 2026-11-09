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

test('V110-F81 an inert or undrawn element is not a Tab stop', async () => {
  const { isStop } = await import('./focusScope.ts');
  assert.equal(isStop(null, true, false), true);
  assert.equal(isStop(null, true, true), false);
  assert.equal(isStop(null, false, false), false);
  assert.equal(isStop('-1', true, false), false);
});

test('V110-F81 Tab skips a stop that refuses focus instead of swallowing the key', async () => {
  const { focusFrom } = await import('./focusScope.ts');
  const g = globalThis as unknown as { document?: { activeElement: unknown } };
  const saved = g.document;
  g.document = { activeElement: null };
  const stop = (takes: boolean) => ({ focus() { if (takes) g.document!.activeElement = this; } });
  try {
    const stops = [stop(false), stop(false), stop(true)];
    assert.equal(focusFrom(stops as never, 0, false), true);
    assert.equal(g.document.activeElement, stops[2]);
    g.document.activeElement = null;
    assert.equal(focusFrom([stop(false), stop(false)] as never, 0, false), false);
    assert.equal(focusFrom([], -1, false), false);
  } finally {
    g.document = saved;
  }
});

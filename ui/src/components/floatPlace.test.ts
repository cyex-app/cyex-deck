import assert from 'node:assert/strict';
import { test } from 'node:test';

import { clampInto, clampSize, recall, remember, startAt } from './floatPlace.ts';

const view = { w: 1000, h: 800 };
const size = { w: 320, h: 400 };

test('V110-N13 a dragged panel stays inside the window', () => {
  assert.deepEqual(clampInto({ x: 100, y: 100 }, size, view), { x: 100, y: 100 });
  assert.deepEqual(clampInto({ x: -50, y: -10 }, size, view), { x: 8, y: 8 });
  assert.deepEqual(clampInto({ x: 900, y: 700 }, size, view), { x: 672, y: 392 });
});

test('V110-N13 the window shrinking pulls the panel back in', () => {
  assert.deepEqual(clampInto({ x: 600, y: 300 }, size, { w: 700, h: 600 }), { x: 372, y: 192 });
});

test('V110-N13 a panel taller than the window keeps its header in view', () => {
  assert.deepEqual(clampInto({ x: 10, y: 50 }, size, { w: 300, h: 300 }), { x: 8, y: 8 });
});

test('V110-N13 panels start centred, the second one offset', () => {
  assert.deepEqual(startAt(size, view), { x: 340, y: 160 });
  assert.deepEqual(startAt(size, view, 1), { x: 368, y: 188 });
});

test('V111-33 a resized panel stays between its minimum and the window edge', () => {
  const min = { w: 300, h: 240 };
  const at = { x: 100, y: 100 };
  assert.deepEqual(clampSize({ w: 500, h: 400 }, min, at, view), { w: 500, h: 400 });
  assert.deepEqual(clampSize({ w: 100, h: 50 }, min, at, view), { w: 300, h: 240 });
  assert.deepEqual(clampSize({ w: 2000, h: 2000 }, min, at, view), { w: 892, h: 692 });
  // A window smaller than the minimum wins over it.
  assert.deepEqual(clampSize({ w: 500, h: 500 }, min, { x: 8, y: 8 }, { w: 200, h: 200 }), { w: 184, h: 184 });
});

test('V111-33 a panel remembers its place by key', () => {
  assert.equal(recall('search:1'), undefined);
  remember('search:1', { at: { x: 5, y: 6 }, size: { w: 7, h: 8 } });
  assert.deepEqual(recall('search:1'), { at: { x: 5, y: 6 }, size: { w: 7, h: 8 } });
});

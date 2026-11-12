import assert from 'node:assert/strict';
import { test } from 'node:test';

import { clampInto, startAt } from './floatPlace.ts';

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

import assert from 'node:assert/strict';
import { test } from 'node:test';

import { ghostAt } from './ghost.ts';

const view = { width: 1000, height: 600 };
const size = { width: 120, height: 26 };

test('the name sits right of and below the pointer when it fits', () => {
  assert.deepEqual(ghostAt({ x: 100, y: 100 }, size, view), { left: 112, top: 110 });
});

test('near the bottom it goes above the pointer', () => {
  assert.deepEqual(ghostAt({ x: 100, y: 590 }, size, view), { left: 112, top: 554 });
});

test('near the right edge it goes left of the pointer', () => {
  assert.deepEqual(ghostAt({ x: 990, y: 100 }, size, view), { left: 858, top: 110 });
});

test('never leaves the window', () => {
  const p = ghostAt({ x: 5, y: 5 }, { width: 2000, height: 900 }, view);
  assert.equal(p.left, 4);
  assert.equal(p.top, 4);
});

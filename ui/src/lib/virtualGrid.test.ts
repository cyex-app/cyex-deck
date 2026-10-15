// Run with `npm run test:ui` (Node's built-in test runner, no extra packages).

import assert from 'node:assert/strict';
import { test } from 'node:test';

import { geometry, range, reveal, step } from './virtualGrid.ts';

const g = geometry(1000, 2000, 176, 48, 6);

test('geometry: 1000 px of 176 px cells is 5 columns (V1051-03)', () => {
  assert.equal(g.columns, 5);
  assert.equal(g.rowHeight, 54);
  assert.equal(g.rows, 400);
  assert.equal(geometry(10, 3, 176, 48, 6).columns, 1);
});

test('range: top, bottom and no height', () => {
  assert.deepEqual(range(g, 2000, 0, 270), { first: 0, last: 34 });
  assert.deepEqual(range(g, 2000, 400 * 54 - 270, 270), { first: 1965, last: 1999 });
  assert.deepEqual(range(g, 2000, 0, 0), { first: 0, last: -1 });
  assert.deepEqual(range(g, 0, 0, 270), { first: 0, last: -1 });
});

test('step: clamps at both ends', () => {
  assert.equal(step(0, 'ArrowLeft', g, 2000, 270), 0);
  assert.equal(step(0, 'ArrowUp', g, 2000, 270), 0);
  assert.equal(step(1999, 'ArrowRight', g, 2000, 270), 1999);
  assert.equal(step(7, 'Home', g, 2000, 270), 0);
  assert.equal(step(7, 'End', g, 2000, 270), 1999);
});

test('step: rows, pages and the last row', () => {
  assert.equal(step(7, 'ArrowDown', g, 2000, 270), 12);
  assert.equal(step(7, 'ArrowUp', g, 2000, 270), 2);
  // 270 / 54 = 5 rows
  assert.equal(step(7, 'PageDown', g, 2000, 270), 32);
  assert.equal(step(32, 'PageUp', g, 2000, 270), 7);
  assert.equal(step(1997, 'ArrowDown', g, 2000, 270), 1997);
  const short = geometry(1000, 12, 176, 48, 6);
  assert.equal(step(8, 'ArrowDown', short, 12, 270), 11);
});

test('reveal: null when in view, else the nearest scrollTop', () => {
  assert.equal(reveal(3, g, 0, 270), null);
  assert.equal(reveal(30, g, 0, 270), 7 * 54 - 270 + 0);
  assert.equal(reveal(0, g, 300, 270), 0);
});

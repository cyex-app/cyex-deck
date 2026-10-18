import assert from 'node:assert/strict';
import { test } from 'node:test';

import { columns, nearest } from './gridNav.ts';

const box = (x: number, y: number, w = 36, h = 36) => ({ left: x, top: y, right: x + w, bottom: y + h });

test('rows: sideways stays in the row, down picks the cell below', () => {
  // Two rows of three.
  const b = [box(0, 0), box(40, 0), box(80, 0), box(0, 40), box(40, 40), box(80, 40)];
  assert.equal(nearest(b, 0, 'right'), 1);
  assert.equal(nearest(b, 2, 'right'), -1);
  assert.equal(nearest(b, 1, 'left'), 0);
  assert.equal(nearest(b, 1, 'down'), 4);
  assert.equal(nearest(b, 4, 'up'), 1);
  assert.equal(nearest(b, 0, 'up'), -1);
  assert.equal(nearest(b, 5, 'down'), -1);
});

test('columns of mixed heights: sideways finds the tile beside it', () => {
  // Left column: tall, short. Right column: short, tall.
  const b = [box(0, 0, 100, 120), box(0, 126, 100, 60), box(106, 0, 100, 60), box(106, 66, 100, 120)];
  assert.equal(nearest(b, 0, 'right'), 2);
  assert.equal(nearest(b, 1, 'right'), 3);
  // The tall right tile goes to whichever left tile sits closest to its middle.
  assert.equal(nearest(b, 3, 'left'), 1);
  assert.equal(nearest(b, 0, 'down'), 1);
  assert.equal(nearest(b, 2, 'down'), 3);
});

test('a missing start is the edge', () => {
  assert.equal(nearest([], 0, 'down'), -1);
});

test('columns: each to the shortest, left first on a tie', () => {
  const r = (x: number) => x;
  // Heights 2, 1, 1, 1: left gets 2, right 1+1, then the tie goes left.
  assert.deepEqual(columns([2, 1, 1, 1], r), [[2, 1], [1, 1]]);
  assert.deepEqual(columns([], r), [[], []]);
});

test('columns: a longer list keeps the split of its start', () => {
  const items = [1.2, 0.5, 0.8, 1, 0.6, 1.4, 0.7, 0.9, 1.1];
  const r = (x: number) => x;
  const a = columns(items.slice(0, 5), r, 2, 0.05);
  const b = columns(items, r, 2, 0.05);
  a.forEach((col, i) => assert.deepEqual(b[i].slice(0, col.length), col));
});

test('columns: a bad ratio counts as square', () => {
  assert.deepEqual(columns([NaN, 0, 1], (x) => x), [[NaN, 1], [0]]);
});

import assert from 'node:assert/strict';
import { test } from 'node:test';

import { geometry } from './virtualGrid.ts';
import { MIN_PAINT_CELL, paintCellWidth } from './paintCell.ts';

test('a short name keeps the 176 px cell (V1051-03)', () => {
  assert.equal(paintCellWidth(0), MIN_PAINT_CELL);
  assert.equal(paintCellWidth(60), MIN_PAINT_CELL);
});

test('a long name widens the cell by its text and padding, rounded up', () => {
  assert.equal(paintCellWidth(200.2), 201 + 34);
  assert.ok(paintCellWidth(140) >= 140 + 30, 'room for the box around the text');
});

test('a wider cell means fewer columns at the same width, never a stretched cell', () => {
  const wide = paintCellWidth(220);
  assert.ok(wide > MIN_PAINT_CELL);
  assert.ok(geometry(1450, 500, wide, 48, 6).columns < geometry(1450, 500, MIN_PAINT_CELL, 48, 6).columns);
});

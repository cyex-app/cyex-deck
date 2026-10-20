// Run with `npm run test:ui` (Node's built-in test runner, no extra packages).

import assert from 'node:assert/strict';
import { test } from 'node:test';

import { MIN_PAINT_CELL, paintCellWidth } from './paintCell.ts';
import { boxWidth, fillCell, geometry, range, reveal, step } from './virtualGrid.ts';

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

test('cells keep their size on every width; only the columns change (V1051-03)', () => {
  // Paint cells (176) and badge cells (72) at 4K/150 % (2560), 1080p (1920) and smaller windows.
  for (const [cellW, widths] of [[176, [1280, 1920, 2560]], [72, [480, 640, 1000]]] as const) {
    const cols = widths.map((w) => geometry(w, 500, cellW, 48, 6).columns);
    assert.ok(cols.every((c, i) => i === 0 || c >= cols[i - 1]));
    assert.ok(new Set(cols).size > 1);
    // The cell is the asked width; leftover space stays at the right edge, under one cell.
    widths.forEach((w, i) => assert.ok(w - (cols[i] * cellW + (cols[i] - 1) * 6) < cellW + 6));
  }
  // The 2560 layout: panes of 1466 and 478 px less the scrollbar slack. 11 paints use the
  // default name width (no long name; 120 px since V107-R5).
  assert.equal(geometry(1466 - 16, 500, paintCellWidth(0), 48, 6).columns, 11);
  assert.equal(paintCellWidth(0), MIN_PAINT_CELL);
  assert.equal(geometry(478 - 16, 500, 72, 76, 6).columns, 6);
});

test('cells keep their width; the box fits the columns and the leftover stays outside', () => {
  for (const w of [300, 700, 1000, 1450]) {
    const x = geometry(w, 50, 176, 48, 6);
    const box = boxWidth(x.columns, 176, 6);
    assert.ok(box <= w || x.columns === 1);
    assert.ok(w - box < 176 + 6);
  }
  assert.equal(boxWidth(5, 176, 6), 904);
});

test('fillCell stretches cells to the width, never below their size', () => {
  // A 313 px chat pane: one paint column of 187 grows to the pane.
  assert.equal(fillCell(313, geometry(313, 10, 187, 48, 6).columns, 187, 6), 313);
  // Four 72 px badges in 313: the leftover is shared.
  const cols = geometry(313, 10, 72, 76, 6).columns;
  assert.equal(cols, 4);
  assert.equal(fillCell(313, cols, 72, 6), 73);
  assert.ok(boxWidth(cols, fillCell(313, cols, 72, 6), 6) <= 313);
  // Narrower than one cell: the cell keeps its size.
  assert.equal(fillCell(100, 1, 187, 6), 187);
});

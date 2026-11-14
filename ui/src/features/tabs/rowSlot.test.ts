import assert from 'node:assert/strict';
import { test } from 'node:test';

import { moveTo } from '../../lib/order.ts';
import { rowSlotAt, sameSlot } from './rowSlot.ts';

// V111-05 Mod mode's button order as drawn: 320 wide, 34 tall, no gaps.
const column = Array.from({ length: 5 }, (_, i) => ({ left: 100, top: 100 + i * 34, width: 320, height: 34 }));
// The same rows with 6 px between them.
const gapped = Array.from({ length: 5 }, (_, i) => ({ left: 100, top: 100 + i * 40, width: 320, height: 34 }));
// Chips that wrap: three on the first line, two on the second, 8 px apart.
const wrapped = [0, 1, 2, 3, 4].map((i) => ({ left: 100 + (i % 3) * 90, top: 100 + Math.floor(i / 3) * 40, width: 80, height: 30 }));

const X = 200;

test('before and after each row, split at its middle (V111-05)', () => {
  column.forEach((r, i) => {
    assert.deepEqual(rowSlotAt(column, X, r.top + 5), { index: i, after: false }, `top of ${i}`);
    const below = rowSlotAt(column, X, r.top + 30);
    // Below a row's middle is before the next one (the same spot), or after the last.
    assert.deepEqual(below, i < column.length - 1 ? { index: i + 1, after: false } : { index: i, after: true }, `bottom of ${i}`);
  });
});

test('a gap between rows goes to the nearer slot, never nowhere (V111-05)', () => {
  // 134..140 is the gap after row 0; anywhere in it is past row 0's middle.
  assert.deepEqual(rowSlotAt(gapped, X, 136), { index: 1, after: false });
  assert.deepEqual(rowSlotAt(gapped, X, 139.5), { index: 1, after: false });
  // Gap after row 3, before the last.
  assert.deepEqual(rowSlotAt(gapped, X, 257), { index: 4, after: false });
});

test('past the end, above the start and beside the list still land (V111-05)', () => {
  const end = column[4].top + column[4].height;
  assert.deepEqual(rowSlotAt(column, X, end + 1), { index: 4, after: true });
  assert.deepEqual(rowSlotAt(column, X, end + 40), { index: 4, after: true });
  assert.deepEqual(rowSlotAt(column, X, 99), { index: 0, after: false });
  assert.deepEqual(rowSlotAt(column, X, 60), { index: 0, after: false });
  // Right of the list (420 + 30), level with row 2's lower half.
  assert.deepEqual(rowSlotAt(column, 450, 190), { index: 3, after: false });
  assert.deepEqual(rowSlotAt(column, 60, 172), { index: 2, after: false });
});

test('far from the list is no slot, so the drop is cancelled (V111-05)', () => {
  assert.equal(rowSlotAt(column, X, 100 + 5 * 34 + 49), null);
  assert.equal(rowSlotAt(column, X, 51), null);
  assert.equal(rowSlotAt(column, 469, 150), null);
  assert.equal(rowSlotAt(column, 51, 150), null);
  assert.equal(rowSlotAt([], X, 150), null);
});

test('a wrapped second line splits by horizontal middles (V111-05)', () => {
  // Second line: rows 3 (100..180) and 4 (190..270), top 140.
  assert.deepEqual(rowSlotAt(wrapped, 110, 150), { index: 3, after: false });
  assert.deepEqual(rowSlotAt(wrapped, 170, 150), { index: 4, after: false });
  assert.deepEqual(rowSlotAt(wrapped, 260, 150), { index: 4, after: true });
  // Past the second line's end, where the first line still has a chip above.
  assert.deepEqual(rowSlotAt(wrapped, 340, 150), { index: 4, after: true });
  // First line's end.
  assert.deepEqual(rowSlotAt(wrapped, 360, 110), { index: 2, after: true });
  // The gap between the lines goes to the nearer one.
  assert.deepEqual(rowSlotAt(wrapped, 110, 132), { index: 0, after: false });
  assert.deepEqual(rowSlotAt(wrapped, 110, 137), { index: 3, after: false });
  // Below everything: the last line.
  assert.deepEqual(rowSlotAt(wrapped, 110, 190), { index: 3, after: false });
});

test('a fast move lands where the pointer is, with nothing in between (V111-05)', () => {
  // One jump from row 0 to far past the end, then back above the start.
  assert.deepEqual(rowSlotAt(column, X, 300), { index: 4, after: true });
  assert.deepEqual(rowSlotAt(column, X, 70), { index: 0, after: false });
  assert.deepEqual(rowSlotAt(column, X, 100 + 3 * 34 + 20), { index: 4, after: false });
});

test('its own slot and the edges next to it change nothing (V111-05)', () => {
  const list = ['a', 'b', 'c', 'd'];
  for (let from = 0; from < list.length; from++) {
    for (let to = 0; to < list.length; to++) {
      for (const after of [false, true]) {
        const same = moveTo(list, from, to, after).every((x, i) => x === list[i]);
        assert.equal(sameSlot(from, to, after), same, `${from} -> ${to} ${after ? 'after' : 'before'}`);
      }
    }
  }
});

test('every slot the column gives moves the row there (V111-05)', () => {
  const list = ['delete', 't60', 'warn', 'ban', 'unban'];
  // Dragging 'delete' (0) down to the lower half of 'ban' (3): after ban.
  const s = rowSlotAt(column, X, column[3].top + 25)!;
  assert.deepEqual(moveTo(list, 0, s.index, s.after), ['t60', 'warn', 'ban', 'delete', 'unban']);
  // Past the end: last.
  const e = rowSlotAt(column, X, 300)!;
  assert.deepEqual(moveTo(list, 1, e.index, e.after), ['delete', 'warn', 'ban', 'unban', 't60']);
  // Above the start: first.
  const f = rowSlotAt(column, X, 80)!;
  assert.deepEqual(moveTo(list, 4, f.index, f.after), ['unban', 'delete', 't60', 'warn', 'ban']);
});

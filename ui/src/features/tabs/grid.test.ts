import assert from 'node:assert/strict';
import { test } from 'node:test';

import { dragSizes, layout, type Split } from '../../lib/panels.ts';
import { gridLine, gridTree, resized } from './grid.ts';

const col = (...ids: string[]) => ({ tiles: ids.map((id) => ({ id })) });

test('F13 columns of tiles laid out like panels, 1 px lines between them', () => {
  const columns = [col('a'), col('b', 'c')];
  const tree = gridTree(columns, { columns: [1, 1], rows: [[1], [1, 3]] }, false)!;
  const laid = layout(tree, { w: 801, h: 403 }, 1);
  assert.deepEqual(laid.panels.get('a'), { x: 0, y: 0, w: 400, h: 403 });
  assert.deepEqual(laid.panels.get('b'), { x: 401, y: 0, w: 400, h: 100 });
  assert.deepEqual(laid.panels.get('c'), { x: 401, y: 101, w: 400, h: 302 });
  assert.equal(laid.dividers.length, 2);
  const [between, inColumn] = laid.dividers;
  assert.deepEqual(gridLine(between, 2), { kind: 'columns' });
  assert.deepEqual(gridLine(inColumn, 2), { kind: 'rows', column: 1 });
});

test('F13 the same tiles in any arrangement: one box each, nothing else changes', () => {
  const before = layout(gridTree([col('a'), col('b'), col('c')], { columns: [1, 1, 1], rows: [] }, false), { w: 900, h: 300 }, 1);
  const after = layout(gridTree([col('b'), col('c'), col('a')], { columns: [1, 1, 1], rows: [] }, false), { w: 900, h: 300 }, 1);
  assert.deepEqual([...before.panels.keys()].sort(), [...after.panels.keys()].sort());
  assert.equal(after.panels.get('a')!.x > after.panels.get('c')!.x, true, 'a moved to the right');
});

test('F13 one column: its dividers are rows; stacked: one under the other', () => {
  const one = gridTree([col('a', 'b')], { columns: [1], rows: [[1, 1]] }, false)!;
  const d = layout(one, { w: 400, h: 401 }, 1).dividers[0];
  assert.deepEqual(gridLine(d, 1), { kind: 'rows', column: 0 });
  const stacked = layout(gridTree([col('a'), col('b'), col('c')], { columns: [3, 1, 1], rows: [] }, true), { w: 300, h: 902 }, 1);
  assert.deepEqual(['a', 'b', 'c'].map((id) => stacked.panels.get(id)!.h), [300, 300, 300]);
  assert.equal(gridTree([], { columns: [], rows: [] }, false), null);
  assert.deepEqual(gridTree([col('a')], { columns: [1], rows: [[1]] }, false), { panel: 'a' });
});

test('F13 a dragged divider keeps the sizes averaging 1', () => {
  const tree = gridTree([col('a'), col('b'), col('c')], { columns: [1, 1, 1], rows: [[1], [1], [1]] }, false) as Split;
  const d = layout(tree, { w: 900, h: 300 }, 1).dividers[0];
  const next = dragSizes(tree, d.rect, d.index, 450, [141, 141]);
  const out = resized({ columns: [1, 1, 1], rows: [[1], [1], [1]] }, gridLine(d, 3), next);
  assert.equal(Math.round(out.columns.reduce((a, b) => a + b, 0) * 1000) / 1000, 3);
  assert.ok(out.columns[0] > 1 && out.columns[1] < 1 && Math.abs(out.columns[2] - 1) < 1e-9);
  assert.deepEqual(out.rows, [[1], [1], [1]]);
});

test('an offline-only column shrinks next to a live one, unless sized by hand', async () => {
  const { slimSizes, SLIM } = await import('./grid.ts');
  const cols = [
    { size: 1, tiles: [{ id: 'a' }] },
    { size: 1, tiles: [{ id: 'b' }] },
  ];
  const sizes = { columns: [1, 1], rows: [[1], [1]] };
  assert.deepEqual(slimSizes(cols, sizes, (id) => id === 'b').columns, [1, SLIM]);
  // Everything offline, or everything live: nothing changes.
  assert.equal(slimSizes(cols, sizes, () => true), sizes);
  assert.equal(slimSizes(cols, sizes, () => false), sizes);
  // A width the user chose stays.
  const sized = { columns: [1.4, 0.6], rows: sizes.rows };
  assert.deepEqual(slimSizes(cols, sized, (id) => id === "b"), sized);
});

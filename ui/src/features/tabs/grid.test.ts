import assert from 'node:assert/strict';
import { test } from 'node:test';

import { dragSizes, layout, type Split } from '../../lib/panels.ts';
import { clampGrid, dragGrid, gridLine, gridTree, resized } from './grid.ts';

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

test('V107-Z1 the last divider of 3+ columns or rows drags to finite shares', () => {
  const tree = gridTree([col('a', 'b', 'c', 'd'), col('e'), col('f', 'g', 'h')], { columns: [1, 1, 1], rows: [[1, 1, 1, 1], [1], [1, 1, 1]] }, false) as Split;
  const laid = layout(tree, { w: 900, h: 1200 }, 1);
  assert.equal(laid.dividers.length, 2 + 3 + 2);
  for (const d of laid.dividers) {
    const s = (d.path.length ? tree.parts[d.path[0]] : tree) as Split;
    for (const at of [-500, 0, 150, 450, 5000]) {
      const next = dragGrid(s, d, at, 141);
      assert.ok(next.every(Number.isFinite), `divider ${d.path}:${d.index} at ${at}: ${next}`);
      assert.ok(Math.abs(next.reduce((a, b) => a + b, 0) - 1) < 1e-9);
    }
  }
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

test('a slim column stays under its cap on a wide window', async () => {
  const { slimSizes, SLIM_MAX_PX } = await import('./grid.ts');
  const cols = [
    { size: 1, tiles: [{ id: 'a' }] },
    { size: 1, tiles: [{ id: 'b' }] },
  ];
  const sizes = { columns: [1, 1], rows: [[1], [1]] };
  const out = slimSizes(cols, sizes, (id) => id === 'b', 2560).columns;
  const share = (out[1] / (out[0] + out[1])) * 2560;
  assert.ok(Math.abs(share - SLIM_MAX_PX) < 0.5, String(share));
  // A narrow window keeps the plain weight.
  assert.deepEqual(slimSizes(cols, sizes, (id) => id === 'b', 1000).columns, [1, 0.3]);
});

test('V107-N6b a tile at 1:3 in a short column keeps its least height; there is room, so the rest is shared as saved', () => {
  const tree = gridTree([col('a'), col('b', 'c')], { columns: [1, 1], rows: [[1], [1, 3]] }, false);
  const min = () => ({ w: 100, h: 200 });
  const laid = layout(clampGrid(tree, { w: 801, h: 451 }, min, 1), { w: 801, h: 451 }, 1);
  assert.equal(laid.panels.get('b')!.h, 200);
  assert.equal(laid.panels.get('c')!.h, 250);
  assert.equal(laid.panels.get('a')!.w, 400, 'columns wide enough keep their shares');
});

test('V107-N6b too short for both tiles: each gets its part; one tile or no room changes nothing', () => {
  const tree = gridTree([col('b', 'c')], { columns: [1], rows: [[1, 3]] }, false);
  const min = () => ({ w: 100, h: 200 });
  const laid = layout(clampGrid(tree, { w: 300, h: 301 }, min, 1), { w: 300, h: 301 }, 1);
  assert.equal(laid.panels.get('b')!.h, laid.panels.get('c')!.h);
  const one = gridTree([col('a')], { columns: [1], rows: [[1]] }, false);
  assert.equal(clampGrid(one, { w: 10, h: 10 }, min), one);
  assert.equal(clampGrid(tree, { w: 0, h: 0 }, min), tree);
});

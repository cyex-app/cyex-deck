import assert from 'node:assert/strict';
import { test } from 'node:test';

import { around, hold, older, oversized, startOf, STEP, WINDOW } from './lineWindow.ts';

const lines = (n: number, first = 0) => Array.from({ length: n }, (_, i) => ({ id: `m${first + i}`, ts: (first + i) * 10 }));

test('newest WINDOW lines are drawn while following (V107-M6)', () => {
  assert.equal(startOf(lines(1000), null), 1000 - WINDOW);
  assert.equal(startOf(lines(100), null), 0);
  assert.equal(startOf([], null), 0);
});

test('a held window keeps its first line as lines come in below', () => {
  const list = lines(1000);
  const from = hold(list, null);
  assert.deepEqual(from, { id: 'm750', ts: 7500 });
  const more = [...list, ...lines(40, 1000)];
  assert.equal(more[startOf(more, from)].id, 'm750');
  assert.equal(hold(more, from), from);
});

test('history put in above stays outside the window', () => {
  const list = lines(300, 500);
  const from = hold(list, null);
  const filled = [...lines(500), ...list];
  assert.equal(filled[startOf(filled, from)].id, from!.id);
  assert.equal(filled.length - startOf(filled, from), WINDOW);
});

test('a first line that dropped off falls to the next one by time', () => {
  const list = lines(1000);
  const from = { id: 'm10', ts: 100 };
  const dropped = list.slice(50);
  assert.equal(startOf(dropped, from), 0);
  const taken = list.filter((m) => m.id !== 'm10');
  assert.equal(taken[startOf(taken, from)].id, 'm11');
});

test('older lines come STEP at a time until all are drawn', () => {
  const list = lines(1000);
  let from = hold(list, null);
  const starts: number[] = [];
  for (let f = older(list, from); f !== undefined; f = older(list, from)) {
    from = f;
    starts.push(startOf(list, from));
  }
  assert.deepEqual(starts, [750 - STEP, 750 - 2 * STEP, 750 - 3 * STEP, 750 - 4 * STEP, 0]);
  assert.equal(older(list, from), undefined);
  assert.equal(older(lines(100), null), undefined);
});

test('a jump to an undrawn line draws it with some above', () => {
  const list = lines(1000);
  assert.equal(around(list, null, 900), undefined);
  const from = around(list, null, 120);
  assert.equal(startOf(list, from!), 70);
  assert.equal(startOf(list, around(list, null, 10)!), 0);
  assert.equal(around(list, null, -1), undefined);
});

test('the window is oversized only past WINDOW lines', () => {
  const list = lines(1000);
  assert.equal(oversized(list, null), false);
  assert.equal(oversized(list, hold(list, null)), false);
  assert.equal(oversized(list, older(list, hold(list, null))!), true);
  assert.equal(oversized(lines(100), { id: 'm0', ts: 0 }), false);
});

import assert from 'node:assert/strict';
import { test } from 'node:test';

import { reconcile } from './reconcile.ts';

const clone = <T>(v: T): T => JSON.parse(JSON.stringify(v)) as T;

/** Runs reconcile on `{ v: cur }` and checks the result equals `next`. */
function land<T>(cur: T, next: T): { v: T } {
  const root = { v: cur };
  reconcile(root, 'v', clone(next));
  assert.deepEqual(root.v, next);
  return root;
}

test('unchanged objects keep their identity', () => {
  const cur = { a: { x: 1 }, b: { y: [1, 2] }, c: 'k' };
  const { a, b } = cur;
  const root = land(cur, { a: { x: 1 }, b: { y: [1, 2] }, c: 'z' });
  assert.equal(root.v, cur);
  assert.equal(root.v.a, a);
  assert.equal(root.v.b, b);
  assert.equal(root.v.b.y, b.y);
});

test('keys are added and removed', () => {
  const cur: Record<string, unknown> = { a: 1, gone: 2 };
  const root = land(cur, { a: 1, added: { n: 3 } });
  assert.equal(root.v, cur);
  assert.equal('gone' in cur, false);
});

test('type changes and null are assigned', () => {
  land<unknown>({ a: { x: 1 } }, { a: null });
  land<unknown>({ a: null }, { a: { x: 1 } });
  land<unknown>({ a: [1] }, { a: { 0: 1 } });
  land<unknown>({ a: 'x' }, { a: [1] });
  land<unknown>(null, { a: 1 });
});

test('rows with ids keep their object when reordered, added or removed', () => {
  const t1 = { id: 'a', name: 'A' };
  const t2 = { id: 'b', name: 'B' };
  const t3 = { id: 'c', name: 'C' };
  const cur = { tabs: [t1, t2, t3] };
  const old = cur.tabs;
  const root = land(cur, { tabs: [t3, { id: 'd', name: 'D' }, { id: 'a', name: 'A2' }] });
  assert.notEqual(root.v.tabs, old);
  assert.deepEqual(old.map((t) => t.id), ['a', 'b', 'c']);
  assert.equal(root.v.tabs[0], t3);
  assert.equal(root.v.tabs[2], t1);
  assert.equal(t1.name, 'A2');
  assert.equal(t2.name, 'B');
});

test('rows with ids in the same order keep the array', () => {
  const list = [{ id: 'a', n: 1 }, { id: 'b', n: 2 }];
  const [a, b] = list;
  const root = land({ list }, { list: [{ id: 'a', n: 1 }, { id: 'b', n: 5 }] });
  assert.equal(root.v.list, list);
  assert.equal(root.v.list[0], a);
  assert.equal(root.v.list[1], b);
});

test('nested rows with ids (tiles in columns) keep their objects', () => {
  const tile = { id: 't', show: { chat: true } };
  const cur = { columns: [{ size: 1, tiles: [tile] }] };
  const col = cur.columns[0];
  const root = land(cur, { columns: [{ size: 2, tiles: [{ id: 't', show: { chat: false } }] }] });
  assert.equal(root.v.columns[0], col);
  assert.equal(root.v.columns[0].tiles[0], tile);
  assert.equal(tile.show.chat, false);
});

test('duplicate or missing ids reconcile by position', () => {
  const list = [{ id: 'a', n: 1 }, { id: 'a', n: 2 }];
  const first = list[0];
  const root = land({ list }, { list: [{ id: 'a', n: 3 }, { id: 'a', n: 2 }] });
  assert.equal(root.v.list, list);
  assert.equal(root.v.list[0], first);
  land({ list: [{ n: 1 }] }, { list: [{ n: 2 }] });
});

test('same length without ids reconciles in place', () => {
  const rows = [{ p: 'x' }, { p: 'y' }];
  const [r0, r1] = rows;
  const root = land({ rows }, { rows: [{ p: 'x' }, { p: 'z' }] });
  assert.equal(root.v.rows, rows);
  assert.equal(root.v.rows[0], r0);
  assert.equal(root.v.rows[1], r1);
});

test('rows without ids coming or going make a new array, leaving the old one as it was', () => {
  const rows = [{ p: 'x' }, { p: 'y' }, { p: 'z' }];
  const gone = rows[1];
  const root = land({ rows }, { rows: [{ p: 'x' }, { p: 'z' }] });
  assert.notEqual(root.v.rows, rows);
  assert.deepEqual(gone, { p: 'y' });
  assert.equal(rows.length, 3);
});

test('equal arrays are left alone', () => {
  const words = ['a', 'b'];
  const root = land({ words }, { words: ['a', 'b'] });
  assert.equal(root.v.words, words);
});

test('empty arrays', () => {
  land({ tabs: [] as { id: string }[] }, { tabs: [{ id: 'a' }] });
  land({ tabs: [{ id: 'a' }] }, { tabs: [] as { id: string }[] });
});

import assert from 'node:assert/strict';
import { test } from 'node:test';

import { byNumber, byText, flip, NAME, pickKey, sortRows, sortWords, TIME, type Sort } from './sort.ts';

const KEYS = [TIME, NAME];
type K = 'time' | 'name';
interface Row {
  name: string;
  at: number;
}
const rows: Row[] = [
  { name: 'beta', at: 2 },
  { name: 'Alpha', at: 3 },
  { name: 'gamma', at: 1 },
  { name: 'alpha2', at: 2 },
];
const by = {
  time: (a: Row, b: Row) => a.at - b.at,
  name: (a: Row, b: Row) => byText(a.name, b.name),
};
const names = (s: Sort<K>) => sortRows(rows, s, by, (a, b) => byText(a.name, b.name)).map((r) => r.name);

test('time down is newest first, ties by name both ways', () => {
  assert.deepEqual(names({ key: 'time', desc: true }), ['Alpha', 'alpha2', 'beta', 'gamma']);
  assert.deepEqual(names({ key: 'time', desc: false }), ['gamma', 'alpha2', 'beta', 'Alpha']);
});

test('A–Z ignores case, numbers by value', () => {
  assert.deepEqual(names({ key: 'name', desc: false }), ['Alpha', 'alpha2', 'beta', 'gamma']);
  assert.deepEqual(names({ key: 'name', desc: true }), ['gamma', 'beta', 'alpha2', 'Alpha']);
  assert.ok(byText('a9', 'a10') < 0);
});

test('the input is left alone', () => {
  const before = rows.map((r) => r.name);
  names({ key: 'name', desc: false });
  assert.deepEqual(
    rows.map((r) => r.name),
    before,
  );
});

test('a new key starts in its own direction, the same key keeps it', () => {
  const s: Sort<K> = { key: 'time', desc: false };
  assert.deepEqual(pickKey(KEYS, s, 'name'), { key: 'name', desc: false });
  assert.deepEqual(pickKey(KEYS, { key: 'name', desc: true }, 'time'), { key: 'time', desc: true });
  assert.equal(pickKey(KEYS, s, 'time'), s);
  assert.deepEqual(flip(s), { key: 'time', desc: true });
});

test('words say what you get', () => {
  assert.equal(sortWords(KEYS, { key: 'time', desc: true }), 'Newest first');
  assert.equal(sortWords(KEYS, { key: 'time', desc: false }), 'Oldest first');
  assert.equal(sortWords(KEYS, { key: 'name', desc: true }), 'Z to A');
  assert.equal(sortWords([], { key: 'x', desc: false }), 'Ascending');
});

test('a comparator can keep rows last both ways', () => {
  const list = [{ v: 0 }, { v: 2 }, { v: null }, { v: 1 }];
  const last = { v: byNumber((x: { v: number | null }) => x.v) };
  assert.deepEqual(sortRows(list, { key: 'v', desc: false }, last).map((x) => x.v), [1, 2, 0, null]);
  assert.deepEqual(sortRows(list, { key: 'v', desc: true }, last).map((x) => x.v), [2, 1, 0, null]);
});

import assert from 'node:assert/strict';
import { test } from 'node:test';
import { mergeLines } from './merge.ts';

const m = (id: string, ts: number, x = '') => ({ id, ts, x });

test('V110-F84 a refresh keeps older pages already shown', () => {
  const shown = [m('a', 1), m('b', 2), m('c', 3), m('d', 4)];
  const fresh = [m('c', 3, 'edited'), m('d', 4), m('e', 5)];
  const out = mergeLines(shown, fresh, 100);
  assert.deepEqual(out.map((x) => x.id), ['a', 'b', 'c', 'd', 'e']);
  assert.equal(out[2].x, 'edited');
});

test('merge keeps only the newest max lines', () => {
  const out = mergeLines([m('a', 1), m('b', 2)], [m('c', 3)], 2);
  assert.deepEqual(out.map((x) => x.id), ['b', 'c']);
});

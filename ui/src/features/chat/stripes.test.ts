import assert from 'node:assert/strict';
import { test } from 'node:test';

import { stripes } from './stripes.ts';

const tinted = (m: Map<string, boolean>) => [...m].filter(([, v]) => v).map(([id]) => id);

test('a fresh list alternates', () => {
  assert.deepEqual(tinted(stripes(['a', 'b', 'c', 'd'], new Map())), ['b', 'd']);
});

test('trimming the top and adding at the bottom never flips a line (V101-01)', () => {
  let s = stripes(['a', 'b', 'c', 'd'], new Map());
  // Paused: three more arrive and the oldest three are trimmed, one batch at a time.
  for (const [drop, add] of [['a', 'e'], ['b', 'f'], ['c', 'g']]) {
    const ids = [...s.keys()].filter((id) => id !== drop).concat(add);
    const before = new Map(s);
    s = stripes(ids, s);
    for (const id of ids) if (before.has(id)) assert.equal(s.get(id), before.get(id), `${id} flipped`);
  }
  assert.deepEqual([...s.keys()], ['d', 'e', 'f', 'g']);
  assert.deepEqual(tinted(s), ['d', 'f']);
});

test('history loaded above alternates against the first line', () => {
  const s = stripes(['x', 'y', 'b', 'c'], stripes(['b', 'c'], new Map()));
  assert.deepEqual([s.get('x'), s.get('y'), s.get('b'), s.get('c')], [false, true, false, true]);
});

test('a line filled in between keeps both neighbours as they were', () => {
  const s = stripes(['a', 'm', 'b'], stripes(['a', 'b'], new Map()));
  assert.equal(s.get('a'), false);
  assert.equal(s.get('b'), true);
  assert.equal(s.get('m'), true);
});

test('lines that are gone are forgotten', () => {
  const s = stripes(['c'], stripes(['a', 'b', 'c'], new Map()));
  assert.deepEqual([...s.keys()], ['c']);
});

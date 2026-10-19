import assert from 'node:assert/strict';
import { test } from 'node:test';

import { nextPiece, prepend } from './historyDrip.ts';

const lines = (n: number, first = 0) => Array.from({ length: n }, (_, i) => ({ id: `m${first + i}` }));
const ids = (l: { id: string }[]) => l.map((m) => m.id);

test('pieces come newest first and keep the order on screen (V107-M1)', () => {
  // Two refill chunks: m50..m99 first, then the older m0..m49.
  const queue = [lines(50, 50), lines(50, 0)];
  let list = lines(3, 100);
  const sizes: number[] = [];
  for (let piece = nextPiece(queue, 30); piece.length; piece = nextPiece(queue, 30)) {
    sizes.push(piece.length);
    list = prepend(list, piece);
  }
  assert.deepEqual(sizes, [30, 20, 30, 20]);
  assert.deepEqual(ids(list), ids(lines(103)));
  assert.equal(queue.length, 0);
});

test('lines already shown are skipped', () => {
  const list = lines(5, 10);
  const out = prepend(list, [...lines(3, 8), { id: 'm10' }]);
  assert.deepEqual(ids(out), ['m8', 'm9', ...ids(list)]);
  assert.equal(prepend(list, lines(2, 10)), list, 'nothing new: same list');
});

test('an empty queue gives nothing', () => {
  assert.deepEqual(nextPiece([[]]), []);
  assert.deepEqual(nextPiece([]), []);
});

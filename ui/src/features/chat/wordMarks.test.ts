import assert from 'node:assert/strict';
import { test } from 'node:test';

import { splitMarked } from './wordMarks.ts';

test('cuts around the marked words', () => {
  assert.deepEqual(splitMarked('love the deck a lot', [{ from: 9, to: 13 }]), [
    { text: 'love the ', hit: false },
    { text: 'deck', hit: true },
    { text: ' a lot', hit: false },
  ]);
});

test('several marks, in any order, at both ends', () => {
  assert.deepEqual(
    splitMarked('ab cd', [
      { from: 3, to: 5 },
      { from: 0, to: 2 },
    ]),
    [
      { text: 'ab', hit: true },
      { text: ' ', hit: false },
      { text: 'cd', hit: true },
    ],
  );
});

test('overlaps and out-of-range marks are cut to fit; no marks leaves the text whole', () => {
  assert.deepEqual(splitMarked('abcd', [{ from: 0, to: 3 }, { from: 2, to: 9 }]), [
    { text: 'abc', hit: true },
    { text: 'd', hit: true },
  ]);
  assert.deepEqual(splitMarked('abcd', [{ from: 7, to: 9 }]), [{ text: 'abcd', hit: false }]);
  assert.deepEqual(splitMarked('abcd', []), [{ text: 'abcd', hit: false }]);
});

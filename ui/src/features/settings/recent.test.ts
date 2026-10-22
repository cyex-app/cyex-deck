import assert from 'node:assert/strict';
import { test } from 'node:test';

import { addRecent, countSearch, removeRecent } from './recent.ts';

test('V107-Mo typing on after a pause replaces what the pause added', () => {
  let l = countSearch(['old'], 'fo')!;
  assert.deepEqual(l, ['fo', 'old']);
  l = countSearch(l, 'font', 'fo')!;
  assert.deepEqual(l, ['font', 'old']);
  // A new search keeps the last one, also when typed over it without clearing.
  assert.deepEqual(countSearch(l, 'dark', ''), ['dark', 'font', 'old']);
  assert.deepEqual(countSearch(l, 'dark', 'font'), ['dark', 'font', 'old']);
  // Cut back after a pause.
  assert.deepEqual(countSearch(['fonts', 'old'], 'font', 'fonts'), ['font', 'old']);
});

test('V107-Mo nothing to save when the list would stay the same', () => {
  assert.deepEqual(countSearch(['font', 'old'], ' Font '), ['Font', 'old']);
  assert.equal(countSearch(['font', 'old'], 'font'), null);
  assert.equal(countSearch(['font', 'old'], 'font', 'font'), null);
  assert.equal(countSearch(['a'], '   '), null);
  assert.deepEqual(countSearch(['a', 'font'], 'font'), ['font', 'a']);
});

test('V107-Mo newest first, no repeats in any case, at most 5', () => {
  let l: string[] = [];
  for (const q of ['a', 'b', 'c', 'd', 'e', 'f']) l = addRecent(l, q);
  assert.deepEqual(l, ['f', 'e', 'd', 'c', 'b']);
  l = addRecent(l, ' D ');
  assert.deepEqual(l, ['D', 'f', 'e', 'c', 'b']);
});

test('V107-Mo empty adds nothing; spaces fold; remove ignores case', () => {
  assert.deepEqual(addRecent(['a'], '   '), ['a']);
  assert.deepEqual(addRecent([], 'dark   mode'), ['dark mode']);
  assert.deepEqual(removeRecent(['Font', 'x'], 'font'), ['x']);
});

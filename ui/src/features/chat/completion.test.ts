import assert from 'node:assert/strict';
import { test } from 'node:test';

import { complete, insert, wordAt } from './completion.ts';

test('the word before the caret', () => {
  assert.deepEqual(wordAt('hi @fo', 6), { start: 3, end: 6, text: '@fo' });
  assert.deepEqual(wordAt('hi ', 3), { start: 3, end: 3, text: '' });
});

test('prefix matches come first, shortest first', () => {
  const list = ['KappaPride', 'Kappa', 'notKappa', 'kappa', 'LUL'];
  assert.deepEqual(complete('kap', list), ['kappa', 'Kappa', 'KappaPride', 'notKappa']);
  assert.deepEqual(complete('', list), []);
  assert.deepEqual(complete('lul', list), []);
});

test('inserting keeps the rest of the text', () => {
  const w = wordAt('hey @fo there', 7);
  assert.deepEqual(insert('hey @fo there', w, '@foo'), { text: 'hey @foo there', caret: 9 });
  const end = wordAt('Kap', 3);
  assert.deepEqual(insert('Kap', end, 'Kappa'), { text: 'Kappa ', caret: 6 });
});

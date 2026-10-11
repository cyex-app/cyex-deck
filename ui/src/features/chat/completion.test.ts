import assert from 'node:assert/strict';
import { test } from 'node:test';

import { complete, insert, rank, RANK, wordAt, wordStarts } from './completion.ts';

test('the word before the caret', () => {
  assert.deepEqual(wordAt('hi @fo', 6), { start: 3, end: 6, text: '@fo' });
  assert.deepEqual(wordAt('hi ', 3), { start: 3, end: 3, text: '' });
});

test('prefix matches come first, shortest first', () => {
  const list = ['KappaPride', 'Kappa', 'notKappa', 'kappa', 'LUL'];
  assert.deepEqual(complete('kap', list), ['kappa', 'Kappa', 'KappaPride', 'notKappa']);
  assert.deepEqual(complete('', list), []);
  // What's typed exactly is left out; another case of it fixes the case.
  assert.deepEqual(complete('LUL', list), []);
  assert.deepEqual(complete('lul', list), ['LUL']);
});

test('V101-16 words split on case, digits and underscores', () => {
  assert.deepEqual(wordStarts('peepoHappy'), [0, 5]);
  assert.deepEqual(wordStarts('KEKW'), [0]);
  assert.deepEqual(wordStarts('forsen1_2Head'), [0, 6, 8, 9]);
  assert.deepEqual(wordStarts('_cat'), [1]);
});

test('V101-16 match-sorter order', () => {
  assert.equal(rank('Pog', 'Pog').rank, RANK.caseEqual);
  assert.equal(rank('Pog', 'pog').rank, RANK.equal);
  assert.equal(rank('PogU', 'pog').rank, RANK.starts);
  assert.equal(rank('widePog', 'pog').rank, RANK.word);
  assert.equal(rank('EZPOGGERS', 'pog').rank, RANK.contains);
  assert.equal(rank('peepoHappy', 'ph').rank, RANK.acronym);
  assert.equal(rank('peepoHappy', 'pphy').rank, RANK.letters);
  assert.equal(rank('peepoHappy', 'x').rank, RANK.none);
  assert.equal(rank('peepoHappy', 'h').rank, RANK.word);
  assert.equal(rank('Kappa', 'k k').rank, RANK.none);
  const list = ['peepoHappy', 'EZPOGGERS', 'widePog', 'PogU', 'pogo', 'pepoG', 'Pog'];
  assert.deepEqual(complete('pog', list), ['Pog', 'pogo', 'PogU', 'widePog', 'pepoG', 'EZPOGGERS']);
  // Closer letters first.
  assert.deepEqual(complete('pg', ['pXXXXg', 'pXg']), ['pXg', 'pXXXXg']);
});

test('V101-16 among equal matches the most used come first', () => {
  const used = (n: string) => ({ KEKWait: 0, KEKL: 1 })[n] ?? Infinity;
  assert.deepEqual(complete('kek', ['KEKW', 'KEKL', 'KEKWait'], 8, used), ['KEKWait', 'KEKL', 'KEKW']);
  assert.deepEqual(complete('kek', ['KEKW', 'KEKL', 'KEKWait']), ['KEKL', 'KEKW', 'KEKWait']);
});

test('inserting keeps the rest of the text', () => {
  const w = wordAt('hey @fo there', 7);
  assert.deepEqual(insert('hey @fo there', w, '@foo'), { text: 'hey @foo there', caret: 9 });
  const end = wordAt('Kap', 3);
  assert.deepEqual(insert('Kap', end, 'Kappa'), { text: 'Kappa ', caret: 6 });
});

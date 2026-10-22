import assert from 'node:assert/strict';
import { test } from 'node:test';

import { complete, insert, rank, rankBest, RANK, wordAt, wordStarts } from './completion.ts';

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
  // V103-13 Settings names have spaces.
  assert.deepEqual(wordStarts('Show timestamps'), [0, 5]);
  assert.deepEqual(wordStarts('7TV name  paints'), [0, 1, 4, 10]);
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

test('inserting keeps the rest of the text', () => {
  const w = wordAt('hey @fo there', 7);
  assert.deepEqual(insert('hey @fo there', w, '@foo'), { text: 'hey @foo there', caret: 9 });
  const end = wordAt('Kap', 3);
  assert.deepEqual(insert('Kap', end, 'Kappa'), { text: 'Kappa ', caret: 6 });
});

test('U5 rankBest takes the highest rank, then the closest', () => {
  assert.deepEqual(rankBest(['zzz', 'Cat'], 'cat'), rank('Cat', 'cat'));
  assert.equal(rankBest(['xyz', 'abc'], 'q').rank, RANK.none);
  assert.deepEqual(rankBest([], 'q'), { rank: RANK.none, close: 0 });
  const near = rank('a_b', 'ab');
  const far = rank('a____b', 'ab');
  assert.equal(near.rank, far.rank);
  assert.deepEqual(rankBest(['a____b', 'a_b'], 'ab'), near);
});

test('V107-MI accents fold: cafe finds café, exact accents rank first', () => {
  assert.equal(rank('café', 'cafe').rank, RANK.equal);
  assert.equal(rank('Café', 'cafe').rank, RANK.equal);
  assert.equal(rank('café', 'CAF').rank, RANK.starts);
  assert.equal(rank('café', 'café').rank, RANK.caseEqual);
  assert.equal(rank('cafe', 'café').rank, RANK.equal);
  assert.equal(rank('lecafé', 'cafe').rank, RANK.contains);
  assert.deepEqual(complete('cafe', ['café', 'cafeteria', 'cafe']), ['café', 'cafeteria']);
  assert.deepEqual(complete('café', ['cafe', 'café2', 'café']), ['cafe', 'café2']);
});

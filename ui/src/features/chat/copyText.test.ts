import assert from 'node:assert/strict';
import { test } from 'node:test';

import { wordAt } from './copyText.ts';

test('V111-59 the word under the click, as copying it gives it', () => {
  const t = 'hello big world';
  assert.equal(wordAt(t, 0), 'hello');
  assert.equal(wordAt(t, 3), 'hello');
  assert.equal(wordAt(t, 5), 'hello', 'just past a word');
  assert.equal(wordAt(t, 6), 'big');
  assert.equal(wordAt(t, 15), 'world', 'the end of the text');
  assert.equal(wordAt(t, 99), 'world');
  assert.equal(wordAt('a  b', 2), '', 'between two spaces');
  assert.equal(wordAt('', 0), '');
  assert.equal(wordAt('https://x.y/z?q=1 ok', 4), 'https://x.y/z?q=1');
});

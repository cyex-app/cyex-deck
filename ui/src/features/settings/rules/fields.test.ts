import assert from 'node:assert/strict';
import { test } from 'node:test';

import { splitList, withAlpha } from './fields.ts';

test('withAlpha keeps the old alpha', () => {
  assert.equal(withAlpha('#112233', '#aabbcc40'), '#11223340');
  assert.equal(withAlpha('#112233', ''), '#1122337f');
});

test('splitList takes commas and spaces', () => {
  assert.deepEqual(splitList(' a, b  c,,'), ['a', 'b', 'c']);
  assert.deepEqual(splitList(''), []);
});

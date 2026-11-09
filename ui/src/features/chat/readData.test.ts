import assert from 'node:assert/strict';
import { test } from 'node:test';
import { readData } from './readData.ts';

test('V110-F84 readData parses JSON and shrugs off the rest', () => {
  assert.deepEqual(readData('{"a":1}'), { a: 1 });
  assert.equal(readData('{oops'), undefined);
  assert.equal(readData(''), undefined);
  assert.equal(readData(undefined), undefined);
});

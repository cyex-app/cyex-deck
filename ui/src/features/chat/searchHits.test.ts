import assert from 'node:assert/strict';
import { test } from 'node:test';
import { searchTerms } from './searchHits.ts';

test('keeps plain words and drops operators', () => {
  assert.deepEqual(searchTerms('from:bob hello World has:link'), ['hello', 'world']);
});
test('joins quoted words', () => {
  assert.deepEqual(searchTerms('"good game" gg'), ['good game', 'gg']);
});
test('operators only gives nothing', () => {
  assert.deepEqual(searchTerms('regex:^a'), []);
});

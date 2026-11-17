import assert from 'node:assert/strict';
import { test } from 'node:test';
import { cutoffOf, everywhereAfter, MAX_HITS, searchCapped, searchTerms, upTo } from './searchHits.ts';

test('keeps plain words and drops operators', () => {
  assert.deepEqual(searchTerms('from:bob hello World has:link'), ['hello', 'world']);
});
test('joins quoted words', () => {
  assert.deepEqual(searchTerms('"good game" gg'), ['good game', 'gg']);
});
test('operators only gives nothing', () => {
  assert.deepEqual(searchTerms('regex:^a'), []);
});

test('searchCapped: only a full answer counts as cut', () => {
  assert.equal(searchCapped(MAX_HITS - 1), false);
  assert.equal(searchCapped(MAX_HITS), true);
});

test('V111-33 the snapshot keeps only lines up to when it was taken', () => {
  const rows = [10, 20, 30].map((ts) => ({ message: { ts } }));
  assert.deepEqual(upTo(rows, 20).map((r) => r.message.ts), [10, 20]);
  assert.deepEqual(upTo(rows, 5), []);
});

test('V111-33 the snapshot time is now, or the newest line if that is later', () => {
  assert.equal(cutoffOf([{ ts: 5 }, { ts: 9 }], 7), 9);
  assert.equal(cutoffOf([{ ts: 5 }], 7), 7);
  assert.equal(cutoffOf([], 7), 7);
});

test('V111-33 a plain Ctrl+F leaves All chats as it is', () => {
  assert.equal(everywhereAfter(true), true);
  assert.equal(everywhereAfter(false), false);
  assert.equal(everywhereAfter(false, true), true);
});

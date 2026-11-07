import assert from 'node:assert/strict';
import { test } from 'node:test';

import { isOwnCard } from './isMe.ts';

test('any signed-in account counts, matched by id', () => {
  assert.equal(isOwnCard('2', ['1', '2', '3']), true);
  assert.equal(isOwnCard('9', ['1', '2', '3']), false);
});
test('the moderating account counts; no profile or empty id never does', () => {
  assert.equal(isOwnCard('7', ['1'], '7'), true);
  assert.equal(isOwnCard(undefined, ['1']), false);
  assert.equal(isOwnCard('', [''], ''), false);
});

import assert from 'node:assert/strict';
import { test } from 'node:test';

import { shouldFold } from './sideFold.ts';

test('a chat under 220 px folds', () => {
  assert.equal(shouldFold(203, 0, true), true);
  assert.equal(shouldFold(204, 0, true), false);
});

test('wide chats keep their buttons', () => {
  assert.equal(shouldFold(400, 2, true), false);
});

test('folds when the box would drop under 120 px', () => {
  // 260 chat: row 244, two buttons and Send leave 244 - 64 - 38 - 4 = 138.
  assert.equal(shouldFold(244, 2, true), false);
  // 230 chat: row 214 leaves 108.
  assert.equal(shouldFold(214, 2, true), true);
});

test('no outside buttons: only the width rule applies', () => {
  assert.equal(shouldFold(210, 0, true), false);
});

test('without Send there is more room', () => {
  assert.equal(shouldFold(214, 2, false), false);
});

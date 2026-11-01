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
  // 290 chat: row 274, two buttons and Send leave 274 - 64 - 38 - 4 = 168.
  assert.equal(shouldFold(274, 2, true), false);
  // 250 chat: row 234 leaves 128.
  assert.equal(shouldFold(234, 2, true), true);
});

test('no outside buttons: only the width rule applies', () => {
  assert.equal(shouldFold(210, 0, true), false);
});

test('without Send there is more room', () => {
  assert.equal(shouldFold(234, 2, false), false);
});

test('buttons inside the box count too', () => {
  // Row 300, Send, three inside buttons: 300 - 38 - 90 - 4 = 168.
  assert.equal(shouldFold(300, 0, true, 3), false);
  // Row 250 leaves 118: the text area would be under 140.
  assert.equal(shouldFold(250, 0, true, 3), true);
});

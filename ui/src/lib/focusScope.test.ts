import assert from 'node:assert/strict';
import { test } from 'node:test';

import { inTabOrder, wrapTab } from './focusScope.ts';

test('Tab wraps at both ends and starts at an end from outside', () => {
  assert.equal(wrapTab(2, 3, false), 0);
  assert.equal(wrapTab(0, 3, true), 2);
  assert.equal(wrapTab(1, 3, false), 2);
  assert.equal(wrapTab(-1, 3, false), 0);
  assert.equal(wrapTab(-1, 3, true), 2);
  assert.equal(wrapTab(0, 0, false), -1);
});

test('a roving tabindex of -1 is out of the Tab order', () => {
  assert.equal(inTabOrder('-1'), false);
  assert.equal(inTabOrder('0'), true);
  assert.equal(inTabOrder(null), true);
});

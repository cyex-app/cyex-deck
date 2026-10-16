import assert from 'node:assert/strict';
import { test } from 'node:test';

import { wrapTab } from './focusScope.ts';

test('Tab wraps at both ends and starts at an end from outside', () => {
  assert.equal(wrapTab(2, 3, false), 0);
  assert.equal(wrapTab(0, 3, true), 2);
  assert.equal(wrapTab(1, 3, false), 2);
  assert.equal(wrapTab(-1, 3, false), 0);
  assert.equal(wrapTab(-1, 3, true), 2);
  assert.equal(wrapTab(0, 0, false), -1);
});

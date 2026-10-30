import assert from 'node:assert/strict';
import { test } from 'node:test';

import { picked, reopenLabel } from './resume.ts';

test('everything is picked until unticked', () => {
  assert.deepEqual(picked(['a', 'b', 'c'], new Set()), ['a', 'b', 'c']);
  assert.deepEqual(picked(['a', 'b', 'c'], new Set(['b'])), ['a', 'c']);
  assert.deepEqual(picked(['a'], new Set(['a'])), []);
});

test('the button says what it does', () => {
  assert.equal(reopenLabel(0), 'Start empty');
  assert.equal(reopenLabel(1), 'Reopen 1 tab');
  assert.equal(reopenLabel(4), 'Reopen 4 tabs');
});

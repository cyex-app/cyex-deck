import assert from 'node:assert/strict';
import { test } from 'node:test';

import { plainLine, sourceOnly } from './sourceOnly.ts';

test('V108-N12 only plain text goes to this channel only', () => {
  assert.equal(plainLine('hello there'), true);
  assert.equal(plainLine('/me waves'), false);
  assert.equal(plainLine('.gg'), false);
  assert.equal(plainLine('  /ban x'), false);
  assert.equal(plainLine('hi', 'msg1'), false);
  assert.equal(plainLine('   '), false);
});

test('V108-N12 the views that send to their own channel only', () => {
  sourceOnly.set(7, true);
  assert.equal(sourceOnly.has(7), true);
  sourceOnly.set(7, false);
  assert.equal(sourceOnly.has(7), false);
  sourceOnly.set(0, true);
  assert.equal(sourceOnly.has(0), false);
});

import { test } from 'node:test';
import assert from 'node:assert/strict';

import { placeFor } from './place.ts';

test('the pick wins', () => {
  assert.equal(placeFor('browser', null, 3), 'browser');
  assert.equal(placeFor('app', '1', 0), 'app');
});

test('first account in the browser, another one in a fresh window', () => {
  assert.equal(placeFor(null, null, 0), 'browser');
  assert.equal(placeFor(null, null, 1), 'app');
  assert.equal(placeFor(null, '1', 1), 'browser');
});

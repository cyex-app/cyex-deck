import { test } from 'node:test';
import assert from 'node:assert/strict';

import { placeFor } from './place.ts';

test('the pick wins', () => {
  assert.equal(placeFor('browser'), 'browser');
  assert.equal(placeFor('app'), 'app');
});

test('unpicked: the browser, for another account too', () => {
  assert.equal(placeFor(null), 'browser');
});

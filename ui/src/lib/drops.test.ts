import assert from 'node:assert/strict';
import { test } from 'node:test';

import { letThrough } from './drops.ts';

test('text into a text field goes through', () => {
  assert.equal(letThrough(['text/plain'], true), true);
});

test('files never go through', () => {
  assert.equal(letThrough(['Files'], true), false);
});

test('anything outside a text field is refused', () => {
  assert.equal(letThrough(['text/uri-list'], false), false);
});

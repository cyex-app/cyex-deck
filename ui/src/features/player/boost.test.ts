import assert from 'node:assert/strict';
import { test } from 'node:test';

import { clampVolume, maxVolume, split } from './boost.ts';

test('up to 100 % the element does the work', () => {
  assert.deepEqual(split(0.4), { element: 0.4, gain: 1 });
  assert.deepEqual(split(1), { element: 1, gain: 1 });
});

test('above 100 % the gain takes the rest', () => {
  assert.deepEqual(split(3.2), { element: 1, gain: 3.2 });
});

test('the boost sets the slider range', () => {
  assert.equal(clampVolume(7, true), 5);
  assert.equal(clampVolume(7, false), 1);
  assert.equal(clampVolume(-1, true), 0);
  assert.equal(maxVolume(false), 1);
});

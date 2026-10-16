import assert from 'node:assert/strict';
import { test } from 'node:test';

import { badgePicture } from './pictures.ts';

const img = ['i1', 'i2', 'i3', 'i4'];
const st = ['s1', 's2', 's3', 's4'];

test('smallest scale that covers the cell', () => {
  assert.equal(badgePicture(img, null, 32, 1, false), 'i2');
  assert.equal(badgePicture(img, null, 32, 1.5, false), 'i3');
  assert.equal(badgePicture(img, null, 32, 2, false), 'i4');
  assert.equal(badgePicture(img, null, 32, 3, false), 'i4');
  assert.equal(badgePicture(img, null, 16, 1, false), 'i1');
});

test('empty slot falls to the next larger, then smaller', () => {
  assert.equal(badgePicture(['i1', '', 'i3', 'i4'], null, 32, 1, false), 'i3');
  assert.equal(badgePicture(['i1', 'i2', '', ''], null, 32, 2, false), 'i2');
  assert.equal(badgePicture(['', '', '', ''], null, 32, 1, false), '');
});

test('still swaps in at the same scale', () => {
  assert.equal(badgePicture(img, st, 32, 1, true), 's2');
  assert.equal(badgePicture(img, st, 32, 1, false), 'i2');
  assert.equal(badgePicture(img, null, 32, 1, true), 'i2');
});

test('fewer pictures than wanted: the largest', () => {
  assert.equal(badgePicture(['i1', 'i2'], null, 32, 3, false), 'i2');
  assert.equal(badgePicture(['i1'], null, 32, 2, false), 'i1');
  assert.equal(badgePicture(['i1', '', ''], null, 32, 2, false), 'i1');
});

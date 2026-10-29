import assert from 'node:assert/strict';
import { test } from 'node:test';

import { rateBusy, rateText } from './rateText.ts';

test('small rates keep one decimal with a comma', () => {
  assert.equal(rateText(3.24), '3,2/s');
  assert.equal(rateText(0.5), '0,5/s');
  assert.equal(rateText(9.96), '10/s');
});

test('big rates round and group with dots', () => {
  assert.equal(rateText(12.6), '13/s');
  assert.equal(rateText(1234), '1.234/s');
});

test('nothing or nonsense reads as zero', () => {
  assert.equal(rateText(0), '0/s');
  assert.equal(rateText(0.04), '0/s');
  assert.equal(rateText(-3), '0/s');
  assert.equal(rateText(Number.NaN), '0/s');
  assert.equal(rateText(Number.POSITIVE_INFINITY), '0/s');
});

test('busy from ten a second', () => {
  assert.equal(rateBusy(9.9), false);
  assert.equal(rateBusy(10), true);
  assert.equal(rateBusy(Number.NaN), false);
});

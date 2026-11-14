import assert from 'node:assert/strict';
import { test } from 'node:test';

import { latencyHigh, latencyText } from './latencyStat.ts';

test('V111-23 latency text uses the decimal comma, empty without a reading', () => {
  assert.equal(latencyText(2.14), '2,1 s');
  assert.equal(latencyText(0), '0,0 s');
  assert.equal(latencyText(null), '');
  assert.equal(latencyText(Number.NaN), '');
});

test('V111-23 high latency warns past 10 s only', () => {
  assert.equal(latencyHigh(10), false);
  assert.equal(latencyHigh(10.1), true);
  assert.equal(latencyHigh(null), false);
  assert.equal(latencyHigh(Number.POSITIVE_INFINITY), false);
});

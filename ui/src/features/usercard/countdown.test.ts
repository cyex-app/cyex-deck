import assert from 'node:assert/strict';
import { test } from 'node:test';

import { timeLeft } from './countdown.ts';

test('timeLeft reads m:ss, then hours, then days, and ends at null', () => {
  assert.equal(timeLeft(272_000, 0), '4:32 left');
  assert.equal(timeLeft(5_000, 0), '0:05 left');
  assert.equal(timeLeft(4_320_000, 0), '1 h 12 min left');
  assert.equal(timeLeft(3_600_000, 0), '1 h left');
  assert.equal(timeLeft(90_000_000, 0), '1 d 1 h left');
  assert.equal(timeLeft(1000, 1000), null);
  assert.equal(timeLeft(0, 5), null);
});

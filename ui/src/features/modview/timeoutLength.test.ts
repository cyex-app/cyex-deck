import assert from 'node:assert/strict';
import { test } from 'node:test';

import { TIMEOUT_MAX, timeoutSecs } from './timeoutLength.ts';

test('V107-R22 typed timeout lengths', () => {
  assert.deepEqual(timeoutSecs('10s'), { secs: 10 });
  assert.deepEqual(timeoutSecs(' 7m '), { secs: 420 });
  assert.deepEqual(timeoutSecs('3h'), { secs: 10_800 });
  assert.deepEqual(timeoutSecs('1d'), { secs: 86_400 });
  assert.deepEqual(timeoutSecs('1h 30m'), { secs: 5400 });
  assert.deepEqual(timeoutSecs('2w'), { secs: TIMEOUT_MAX });
  assert.deepEqual(timeoutSecs('90'), { secs: 90 }, 'a bare number is seconds');
});

test('V107-R22 what Twitch refuses is said, not sent', () => {
  for (const bad of ['', 'abc', '0', '5x', '-3m']) assert.ok('error' in timeoutSecs(bad), bad);
  assert.deepEqual(timeoutSecs('15d'), { error: 'Twitch allows up to 14 days.' });
  assert.deepEqual(timeoutSecs('14d 1s'), { error: 'Twitch allows up to 14 days.' });
});

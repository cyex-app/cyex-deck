import assert from 'node:assert/strict';
import { test } from 'node:test';

import { durationError, timeoutSecs, toSeconds } from './massDuration.ts';

test('V107-R21b units convert to seconds', () => {
  assert.equal(toSeconds(30, 'seconds'), 30);
  assert.equal(toSeconds(10, 'minutes'), 600);
  assert.equal(toSeconds(2, 'hours'), 7200);
  assert.equal(toSeconds(14, 'days'), 1209600);
});

test('V107-R21b 0 or empty is a ban', () => {
  assert.equal(timeoutSecs(0, 'hours'), null);
  assert.equal(timeoutSecs(Number.NaN, 'days'), null);
  assert.equal(timeoutSecs(5, 'minutes'), 300);
  assert.equal(durationError(0, 'minutes'), null);
});

test('V107-R21b 1 s to 14 days is allowed, more gets a short error', () => {
  assert.equal(durationError(1, 'seconds'), null);
  assert.equal(durationError(14, 'days'), null);
  assert.equal(durationError(336, 'hours'), null);
  assert.equal(durationError(15, 'days'), 'Twitch allows up to 14 days.');
  assert.equal(durationError(1209601, 'seconds'), 'Twitch allows up to 14 days.');
});

import assert from 'node:assert/strict';
import { test } from 'node:test';
import { AD_NOTE, adClock, endsAt, msLeft } from './adCountdown.ts';

test('adClock rounds up and pads seconds', () => {
  assert.equal(adClock(42_000), '0:42');
  assert.equal(adClock(41_001), '0:42');
  assert.equal(adClock(61_000), '1:01');
  assert.equal(adClock(600_000), '10:00');
  assert.equal(adClock(0), '0:00');
  assert.equal(adClock(-5), '0:00');
});

test('endsAt needs a positive finite ms', () => {
  assert.equal(endsAt(30_000, 1000), 31_000);
  assert.equal(endsAt(null, 1000), null);
  assert.equal(endsAt(undefined, 1000), null);
  assert.equal(endsAt(0, 1000), null);
  assert.equal(endsAt(Number.NaN, 1000), null);
});

test('counts down from the end and stops at zero', () => {
  const end = endsAt(24_000, 0);
  assert.equal(msLeft(end, 4_000), 20_000);
  assert.equal(msLeft(end, 99_000), 0);
  assert.equal(msLeft(null, 5), null);
});

test('the plain note stays for no duration', () => {
  assert.ok(AD_NOTE.startsWith('Ad break.'));
});

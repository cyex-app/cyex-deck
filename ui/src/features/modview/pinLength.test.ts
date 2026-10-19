import assert from 'node:assert/strict';
import { test } from 'node:test';

import { pinCustomSecs } from './pinLength.ts';

test('V107 a custom pin length: minutes, m:ss or units', () => {
  assert.equal(pinCustomSecs('12'), 720);
  assert.equal(pinCustomSecs(' 2:30 '), 150);
  assert.equal(pinCustomSecs('1,5'), 90);
  assert.equal(pinCustomSecs('90s'), 90);
  assert.equal(pinCustomSecs('1m 30s'), 90);
});

test('V107 clamped to what Twitch takes, 30 s to 30 min', () => {
  assert.equal(pinCustomSecs('10s'), 30);
  assert.equal(pinCustomSecs('0:05'), 30);
  assert.equal(pinCustomSecs('45'), 1800);
  assert.equal(pinCustomSecs('2h'), 1800);
  assert.equal(pinCustomSecs('30:00'), 1800);
});

test('V107 junk and zero are refused', () => {
  for (const t of ['', '  ', 'soon', '0', '0:00', '1:75', '5x', '-3']) assert.equal(pinCustomSecs(t), null, t);
});

import assert from 'node:assert/strict';
import { test } from 'node:test';

import { adSecs, followMinutes, slowSecs, withPreset } from './modePresets.ts';

test('V110-M23 slow mode: 3 s to 2 min', () => {
  assert.deepEqual(slowSecs('45'), { value: 45 });
  assert.deepEqual(slowSecs('2m'), { value: 120 });
  for (const t of ['', '2', '3m', 'soon']) assert.ok('error' in slowSecs(t), t);
});

test('V110-M23 followers only: bare numbers are minutes', () => {
  assert.deepEqual(followMinutes('30'), { value: 30 });
  assert.deepEqual(followMinutes('1d'), { value: 1440 });
  assert.deepEqual(followMinutes('2w'), { value: 20_160 });
  assert.deepEqual(followMinutes('3mo'), { value: 129_600 });
  for (const t of ['', '0', '30s', '4mo', 'x']) assert.ok('error' in followMinutes(t), t);
});

test('V110-M23 ads: steps of 30 s up to 3 min', () => {
  assert.deepEqual(adSecs('90'), { value: 90 });
  assert.deepEqual(adSecs('2m'), { value: 120 });
  for (const t of ['', '45', '210', '4m']) assert.ok('error' in adSecs(t), t);
});

test('V110-M23 a preset joins sorted, once, and the list stays short', () => {
  assert.deepEqual(withPreset([10, 30], 20), [10, 20, 30]);
  assert.deepEqual(withPreset([10, 30], 30), [10, 30]);
  assert.equal(withPreset([1, 2, 3, 4, 5, 6, 7, 8, 9, 10], 11).length, 10);
});

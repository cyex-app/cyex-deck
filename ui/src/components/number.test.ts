import assert from 'node:assert/strict';
import { test } from 'node:test';

import { clampNumber, parseNumber, stepFor } from './number.ts';

const r = { min: 0, max: 100 };

test('clamps and snaps to the step', () => {
  assert.equal(clampNumber(150, r), 100);
  assert.equal(clampNumber(-3, r), 0);
  assert.equal(clampNumber(7.4, r), 7);
  assert.equal(clampNumber(0.30000000000000004, { min: 0, max: 1, step: 0.1 }), 0.3);
  assert.equal(clampNumber(12, { min: 5, max: 60, step: 5 }), 10);
});

test('reads typed numbers, comma decimals and dot groups', () => {
  assert.equal(parseNumber(' 42 ', r), 42);
  assert.equal(parseNumber('1,5', { min: 0, max: 10, step: 0.5 }), 1.5);
  assert.equal(parseNumber('1.000', { min: 0, max: 5000 }), 1000);
  assert.equal(parseNumber('2.5', { min: 0, max: 10, step: 0.5 }), 2.5);
  assert.equal(parseNumber('abc', r), null);
  assert.equal(parseNumber('', r), null);
  assert.equal(parseNumber('999', r), 100);
});

test('keys step, Shift and Page keys by ten, Home and End jump', () => {
  assert.equal(stepFor('ArrowUp', false, 5, r), 6);
  assert.equal(stepFor('ArrowDown', true, 5, r), 0);
  assert.equal(stepFor('PageUp', false, 95, r), 100);
  assert.equal(stepFor('Home', false, 50, r), 0);
  assert.equal(stepFor('End', false, 50, r), 100);
  assert.equal(stepFor('a', false, 50, r), null);
});

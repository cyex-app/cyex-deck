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

test('parseNumber: words and empty text are not numbers; out of range clamps', () => {
  const r = { min: 1, max: 2147483647 };
  assert.equal(parseNumber('abc', r), null);
  assert.equal(parseNumber('', r), null);
  assert.equal(parseNumber('0', r), 1);
  assert.equal(parseNumber('99999999999', r), 2147483647);
});

import { rangeNote, readNumber } from './number.ts';

const g = (n: number) => String(n).replace(/\B(?=(\d{3})+(?!\d))/g, '.');
const kept = { min: 100, max: 8000 };

test('messages-kept range: dots, empty, huge, negative, decimals', () => {
  assert.equal(parseNumber('1.000', kept), 1000);
  assert.equal(parseNumber('1000', kept), 1000);
  assert.equal(parseNumber('', kept), null);
  assert.equal(parseNumber('   ', kept), null);
  assert.equal(parseNumber('99999999999999', kept), 8000);
  assert.equal(parseNumber('-5', kept), 100);
  assert.equal(parseNumber('250,7', kept), 251);
  assert.equal(parseNumber('250.4', kept), 250);
  assert.equal(parseNumber('1e999', kept), null);
});

test('readNumber is unclamped', () => {
  assert.equal(readNumber('9.000'), 9000);
  assert.equal(readNumber('x'), null);
  assert.equal(readNumber(''), null);
});

test('rangeNote names the limit that was passed', () => {
  assert.equal(rangeNote('9000', kept, g), 'Max 8.000');
  assert.equal(rangeNote('9.000', kept, g), 'Max 8.000');
  assert.equal(rangeNote('5', kept, g), 'Min 100');
  assert.equal(rangeNote('-5', kept, g), 'Min 100');
  assert.equal(rangeNote('500', kept, g), null);
  assert.equal(rangeNote('abc', kept, g), null);
  assert.equal(rangeNote('', kept, g), null);
});

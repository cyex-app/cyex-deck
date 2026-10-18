import assert from 'node:assert/strict';
import { test } from 'node:test';

import { matchTyped, moveFor, radioKey } from './select.ts';

const c = ['Speakers', 'Headphones', 'Headset', 'Monitor'].map((l) => ({ value: l, label: l }));

test('arrows stop at the ends, Home and End jump', () => {
  assert.equal(moveFor('ArrowDown', 3, 4), 3);
  assert.equal(moveFor('ArrowUp', 0, 4), 0);
  assert.equal(moveFor('Home', 2, 4), 0);
  assert.equal(moveFor('End', 0, 4), 3);
  assert.equal(moveFor('a', 0, 4), null);
  assert.equal(moveFor('End', 0, 0), null);
});

test('type-ahead finds a prefix and a repeated letter cycles', () => {
  assert.equal(matchTyped(c, 'm', 0), 3);
  assert.equal(matchTyped(c, 'hea', 0), 1);
  assert.equal(matchTyped(c, 'h', 0), 1);
  assert.equal(matchTyped(c, 'h', 1), 2);
  assert.equal(matchTyped(c, 'h', 2), 1);
  assert.equal(matchTyped(c, 'x', 0), -1);
});

test('type-ahead ignores pictures and still matches the label', () => {
  const p = [
    { value: '1', label: 'alice', picture: 'a.png' },
    { value: '2', label: 'Bob', picture: null },
  ];
  assert.equal(matchTyped(p, 'b', 0), 1);
});

test('radio keys: both arrow pairs step and wrap, Home and End jump', () => {
  assert.equal(radioKey({ key: 'ArrowRight' }, 1, 4), 2);
  assert.equal(radioKey({ key: 'ArrowDown' }, 3, 4), 0);
  assert.equal(radioKey({ key: 'ArrowLeft' }, 0, 4), 3);
  assert.equal(radioKey({ key: 'ArrowUp' }, 2, 4), 1);
  assert.equal(radioKey({ key: 'Home' }, 2, 4), 0);
  assert.equal(radioKey({ key: 'End' }, 0, 4), 3);
  assert.equal(radioKey({ key: 'a' }, 0, 4), null);
  assert.equal(radioKey({ key: 'ArrowRight' }, 0, 0), null);
});

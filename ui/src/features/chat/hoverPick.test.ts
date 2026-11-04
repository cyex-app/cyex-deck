import assert from 'node:assert/strict';
import { test } from 'node:test';

import { Pick } from './hoverPick.ts';

const A = Symbol('a');
const B = Symbol('b');

test('within a list one bar at a time: the pick wins over the pointer', () => {
  const p = new Pick();
  assert.equal(p.hot('1', A, '1'), true);
  assert.deepEqual(p.set(A, '2'), ['2']);
  assert.equal(p.hot('2', A, '1'), true);
  assert.equal(p.hot('1', A, '1'), false);
  assert.deepEqual(p.set(A, '3'), ['2', '3']);
});

test('a pick in one list leaves the other lists to the pointer', () => {
  const p = new Pick();
  p.set(A, '2');
  assert.equal(p.hot('9', B, '9'), true);
  assert.equal(p.hot('2', B, null), false);
  assert.equal(p.hot('9', undefined, '9'), true);
});

test('a list lets go of its own pick only', () => {
  const p = new Pick();
  p.set(A, '2');
  assert.deepEqual(p.set(B, null), []);
  assert.equal(p.id, '2');
  assert.deepEqual(p.set(A, null), ['2']);
  assert.equal(p.id, null);
  assert.equal(p.hot('1', A, '1'), true);
});

test('a second list taking over replaces the first pick', () => {
  const p = new Pick();
  p.set(A, '2');
  assert.deepEqual(p.set(B, '7'), ['2', '7']);
  assert.equal(p.holds(A), false);
  assert.equal(p.holds(B), true);
});

test('V108-F60a picks() is true for the keyboard pick only, never for hover', () => {
  const p = new Pick();
  assert.equal(p.picks('1', A), false);
  p.set(A, '2');
  assert.equal(p.picks('2', A), true);
  assert.equal(p.picks('2', B), false);
  assert.equal(p.picks('1', A), false);
  assert.equal(p.picks('2', undefined), false);
});

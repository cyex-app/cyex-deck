import assert from 'node:assert/strict';
import { test } from 'node:test';

import { DRAFT, stayOpen } from './stay.ts';

const inside = {};
const outside = {};
const box = (draft: boolean) => ({
  contains: (n: unknown) => n === inside,
  querySelector: (s: string) => (draft && s === DRAFT ? {} : null),
});

test('wheel over the card keeps it', () => {
  assert.equal(stayOpen(box(false), { type: 'wheel', target: inside }), true);
});

test('wheel outside an untouched card closes it', () => {
  assert.equal(stayOpen(box(false), { type: 'wheel', target: outside }), false);
});

test('a lost window closes an untouched card', () => {
  assert.equal(stayOpen(box(false), { type: 'blur', target: null }), false);
});

test('a draft keeps it on wheel anywhere and on blur', () => {
  assert.equal(stayOpen(box(true), { type: 'wheel', target: outside }), true);
  assert.equal(stayOpen(box(true), { type: 'blur', target: null }), true);
});

test('no box: nothing to keep', () => {
  assert.equal(stayOpen(undefined, { type: 'wheel', target: inside }), false);
});

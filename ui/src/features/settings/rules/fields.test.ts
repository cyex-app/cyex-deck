import assert from 'node:assert/strict';
import { test } from 'node:test';

import { applyEdit, splitList, withAlpha } from './fields.ts';

test('withAlpha keeps the old alpha', () => {
  assert.equal(withAlpha('#112233', '#aabbcc40'), '#11223340');
  assert.equal(withAlpha('#112233', ''), '#1122337f');
});

test('splitList takes commas and spaces', () => {
  assert.deepEqual(splitList(' a, b  c,,'), ['a', 'b', 'c']);
  assert.deepEqual(splitList(''), []);
});

test('applyEdit keeps an edit made in the same tick (b-10)', () => {
  const seen = { name: '', sends: '', raw: '' };
  const now = { name: '/hi', sends: '', raw: '/hi' };
  const next = { ...seen, sends: 'Hello' };
  assert.deepEqual(applyEdit(now, seen, next), { name: '/hi', sends: 'Hello', raw: '/hi' });
  assert.deepEqual(applyEdit(now, now, { ...now, name: '/yo' }), { name: '/yo', sends: '', raw: '/hi' });
});

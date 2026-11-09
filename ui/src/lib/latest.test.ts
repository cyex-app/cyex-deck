import assert from 'node:assert/strict';
import { test } from 'node:test';

import { latest } from './latest.ts';

test('only the newest request is current', () => {
  const l = latest();
  const a = l.begin();
  assert.equal(a(), true);
  const b = l.begin();
  assert.equal(a(), false);
  assert.equal(b(), true);
});

test('cancel drops what is in flight', () => {
  const l = latest();
  const a = l.begin();
  l.cancel();
  assert.equal(a(), false);
});

test('peek follows the current request without replacing it', () => {
  const l = latest();
  const a = l.begin();
  const p = l.peek();
  assert.equal(a(), true);
  assert.equal(p(), true);
  const b = l.begin();
  assert.equal(p(), false);
  assert.equal(b(), true);
  const q = l.peek();
  l.cancel();
  assert.equal(q(), false);
});

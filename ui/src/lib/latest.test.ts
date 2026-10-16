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

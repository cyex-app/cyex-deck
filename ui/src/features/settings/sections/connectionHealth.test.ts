import assert from 'node:assert/strict';
import { test } from 'node:test';

import { ago, healthRows } from './connectionHealth.ts';

test('V110-U65 the age is worded', () => {
  assert.equal(ago(42_000), '42 s ago');
  assert.equal(ago(5 * 60_000), '5 min ago');
  assert.equal(ago(3 * 3_600_000), '3 h ago');
  assert.equal(ago(-5), '0 s ago');
});

test('V110-U65 blanks become a dash', () => {
  const r = healthRows([{ label: '7TV', count: 2, lastAt: 0, lastError: '' }], 1000);
  assert.deepEqual(r[0], { label: '7TV', count: 2, when: '–', why: '–' });
});

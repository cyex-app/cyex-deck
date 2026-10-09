import assert from 'node:assert/strict';
import { test } from 'node:test';

import { clock, uptime } from './time.ts';

test('clocks', () => {
  assert.equal(clock(0), '0:00');
  assert.equal(clock(125.9), '2:05');
  assert.equal(clock(3725), '1:02:05');
  assert.equal(clock(Number.NaN), '0:00');
});

test('uptime from the start', () => {
  assert.equal(uptime('2026-10-09T10:00:00Z', Date.parse('2026-10-09T12:30:05Z')), '2:30:05');
  assert.equal(uptime('nope'), '');
});

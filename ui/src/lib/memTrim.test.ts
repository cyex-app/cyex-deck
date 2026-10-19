import assert from 'node:assert/strict';
import { test } from 'node:test';

import { TRIM_IDLE_MS, trimStep } from './memTrim.ts';

test('L2 trims only a shown, idle window', () => {
  const now = 1_000_000;
  assert.equal(trimStep(now, now - TRIM_IDLE_MS, false), 'trim');
  assert.equal(trimStep(now, now - TRIM_IDLE_MS + 1, false), 'retry');
  assert.equal(trimStep(now, now, false), 'retry');
  assert.equal(trimStep(now, 0, true), 'wait');
});

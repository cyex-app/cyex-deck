import assert from 'node:assert/strict';
import { test } from 'node:test';

import { CONTROL_SELECTOR, litTimes } from './revealPlan.ts';

test('V110-U56 the tint fades over about a second, and not at all with reduced motion', () => {
  const normal = litTimes(false);
  assert.ok(normal.fadeMs >= 900 && normal.fadeMs <= 1100);
  assert.ok(normal.holdMs > 0);
  assert.equal(litTimes(true).fadeMs, 0);
});

test('V110-U56 focus goes to real controls only', () => {
  assert.ok(CONTROL_SELECTOR.includes('input'));
  assert.ok(CONTROL_SELECTOR.includes(':not(:disabled)'));
});

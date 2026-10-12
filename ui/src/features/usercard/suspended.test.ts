import assert from 'node:assert/strict';
import { test } from 'node:test';

import { suspendedFlag } from './suspended.ts';

test('a flag for each reason, none for a normal account', () => {
  assert.equal(suspendedFlag('temporary')?.label, 'Suspended · temporary');
  assert.equal(suspendedFlag('indefinite')?.label, 'Suspended · indefinite');
  assert.equal(suspendedFlag('dmca')?.label, 'Suspended · DMCA');
  assert.equal(suspendedFlag('deactivated')?.label, 'Deactivated');
  assert.equal(suspendedFlag('other')?.label, 'Hidden on Twitch');
  assert.equal(suspendedFlag(null), null);
  assert.equal(suspendedFlag(undefined), null);
});

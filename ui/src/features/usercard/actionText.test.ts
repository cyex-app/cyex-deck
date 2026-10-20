import { test } from 'node:test';
import assert from 'node:assert/strict';
import { actionText } from './actionText.ts';

test('timeout lengths read like chat', () => {
  assert.equal(actionText('cyexbot timed out caressz for 600 seconds'), 'cyexbot timed out caressz for 10m');
  assert.equal(actionText('M timed out U for 1 second'), 'M timed out U for 1s');
  assert.equal(
    actionText('M timed out U for 90 seconds, with reason: wait for 5 seconds'),
    'M timed out U for 1m 30s, with reason: wait for 5 seconds',
  );
  assert.equal(actionText('Mod1 timed out U for 10m'), 'Mod1 timed out U for 10m');
  assert.equal(actionText('Banned'), 'Banned');
});

import assert from 'node:assert/strict';
import { test } from 'node:test';

import { massConfirm } from './massConfirm.ts';

test('V110-A2 one name or none goes straight through', () => {
  assert.equal(massConfirm(0, false), null);
  assert.equal(massConfirm(1, false), null);
  assert.equal(massConfirm(1, true), null);
  assert.equal(massConfirm(Number.NaN, false), null);
});

test('V110-A2 more than one name asks, with a grouped count', () => {
  assert.equal(massConfirm(2, false), 'Ban 2?');
  assert.equal(massConfirm(480, true), 'Time out 480?');
  assert.equal(massConfirm(1520, false), 'Ban 1.520?');
});

test('V110-A2 the panel asks through ConfirmButton when more than one name is listed', async () => {
  const { readFileSync } = await import('node:fs');
  const src = readFileSync(new URL('./MassActions.svelte', import.meta.url), 'utf8');
  assert.match(src, /massConfirm\(logins\.length, amount > 0\)/);
  assert.match(src, /confirm=\{goAsk\}/);
});

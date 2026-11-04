import assert from 'node:assert/strict';
import { test } from 'node:test';

import { waitGone } from './waitGone.ts';

test('V108-F59a waitGone resolves true when the condition turns true', async () => {
  let n = 0;
  const r = await waitGone(() => ++n > 3, 1000, () => 0, (cb) => cb());
  assert.equal(r, true);
});

test('V108-F59a waitGone gives up after the limit', async () => {
  let t = 0;
  const r = await waitGone(() => false, 100, () => (t += 40), (cb) => cb());
  assert.equal(r, false);
});

test('V108-F59a waitGone is true at once if already gone', async () => {
  assert.equal(await waitGone(() => true), true);
});

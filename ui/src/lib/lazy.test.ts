import assert from 'node:assert/strict';
import { test } from 'node:test';

import { lazy } from './lazy.ts';

test('loads once, on first use', async () => {
  let calls = 0;
  const get = lazy(async () => ++calls);
  assert.equal(calls, 0);
  assert.equal(await get(), 1);
  assert.equal(await get(), 1);
  assert.equal(calls, 1);
});

test('a failed load is tried again on the next use', async () => {
  let calls = 0;
  const get = lazy(async () => {
    if (++calls === 1) throw new Error('gone');
    return calls;
  });
  await assert.rejects(get());
  assert.equal(await get(), 2);
  assert.equal(await get(), 2);
});

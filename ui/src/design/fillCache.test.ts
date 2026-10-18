import assert from 'node:assert/strict';
import { test } from 'node:test';

import { fillCache } from './fillCache.ts';

test('asks once per accent and base, returns the adjusted fill', async () => {
  const asked: string[] = [];
  const fill = fillCache(async (accent, theme) => {
    asked.push(`${theme}|${accent}`);
    return '#5a5a5a';
  });
  assert.equal(await fill('#101010', 'dark'), '#5a5a5a');
  assert.equal(await fill('#101010', 'dark'), '#5a5a5a');
  assert.equal(await fill('#101010', 'light'), '#5a5a5a');
  assert.deepEqual(asked, ['dark|#101010', 'light|#101010']);
});

test('falls back to the raw accent on failure and asks again later', async () => {
  let fail = true;
  const fill = fillCache(async () => {
    if (fail) throw new Error('no');
    return '#8a8a8a';
  });
  assert.equal(await fill('#f0f0f0', 'light'), '#f0f0f0');
  fail = false;
  assert.equal(await fill('#f0f0f0', 'light'), '#8a8a8a');
});

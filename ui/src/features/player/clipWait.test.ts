import assert from 'node:assert/strict';
import { test } from 'node:test';

import { afterAsk, FAILED_TEXT, GONE_TEXT, MAKING_FRESH, MAKING_OTHER, READY_TEXT, SLOW_TEXT, UNFINISHED_TEXT } from './clipWait.ts';

test('V109-F73 a clip still being made is waited for, never called gone', () => {
  // Before: 10 asks 3 s apart, then "This clip is gone." while Twitch still said CREATING.
  for (let t = 1; t < MAKING_FRESH; t++) assert.equal(afterAsk('making', t, true).retry, true, `ask ${t}`);
  assert.equal(afterAsk('making', 2, true).text, READY_TEXT);
  assert.equal(afterAsk('making', 12, true).text, SLOW_TEXT, 'slow: says so');
  assert.deepEqual(afterAsk('making', MAKING_FRESH, true), { retry: false, text: UNFINISHED_TEXT });
  assert.deepEqual(afterAsk('making', MAKING_OTHER, false), { retry: false, text: UNFINISHED_TEXT });
});

test('V109-F73 a failed clip says so at once', () => {
  assert.deepEqual(afterAsk('failed', 1, true), { retry: false, text: FAILED_TEXT });
});

test('gone: a clip made here gets a few asks, others none', () => {
  assert.deepEqual(afterAsk('gone', 1, true), { retry: true, text: READY_TEXT });
  assert.deepEqual(afterAsk('gone', 5, true), { retry: false, text: GONE_TEXT });
  assert.deepEqual(afterAsk('gone', 1, false), { retry: false, text: GONE_TEXT });
});

test('errors: retried for a fresh clip, else the caller words them', () => {
  assert.equal(afterAsk('error', 3, true).retry, true);
  assert.deepEqual(afterAsk('error', 10, true), { retry: false, text: null });
  assert.deepEqual(afterAsk('error', 1, false), { retry: false, text: null });
  assert.deepEqual(afterAsk('ready', 1, true), { retry: false, text: null });
});

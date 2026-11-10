import assert from 'node:assert/strict';
import { test } from 'node:test';

import { MANIFEST_POLICY, QUICK_TRIES, RECHECK_MS, offlineDelay, recheckDelay } from './recheck.ts';

const base = { failure: 'network', tries: QUICK_TRIES, listed: true, hidden: false };

test('waits 15 s once the quick tries are used up', () => {
  assert.equal(recheckDelay(base), RECHECK_MS);
});
test('leaves the quick tries alone', () => {
  assert.equal(recheckDelay({ ...base, tries: QUICK_TRIES - 1 }), null);
});
test('stops when the stream is no longer listed', () => {
  assert.equal(recheckDelay({ ...base, listed: false }), null);
});
test('stops while the page is hidden', () => {
  assert.equal(recheckDelay({ ...base, hidden: true }), null);
});
test('only rechecks network failures', () => {
  for (const failure of [null, 'offline', 'geo', 'subs', 'media', 'gone']) {
    assert.equal(recheckDelay({ ...base, failure }), null);
  }
});

test('V110-F83 offline rechecks back off 15, 30, 60 s and rest while hidden', () => {
  assert.equal(offlineDelay(0, false), 15_000);
  assert.equal(offlineDelay(1, false), 30_000);
  assert.equal(offlineDelay(2, false), 60_000);
  assert.equal(offlineDelay(9, false), 60_000);
  assert.equal(offlineDelay(0, true), null);
});

test('V110-F96 a failed master playlist is not asked again by hls.js itself', () => {
  const d = MANIFEST_POLICY!.default!;
  assert.equal(d.errorRetry.maxNumRetry, 0);
  assert.ok(d.timeoutRetry.maxNumRetry <= 1);
});

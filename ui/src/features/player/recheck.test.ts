import assert from 'node:assert/strict';
import { test } from 'node:test';

import { QUICK_TRIES, RECHECK_MS, recheckDelay } from './recheck.ts';

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

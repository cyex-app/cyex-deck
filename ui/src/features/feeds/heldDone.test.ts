import assert from 'node:assert/strict';
import { test } from 'node:test';

import { heldDone } from './heldDone.ts';

test('V111-17 heldDone says who decided, or nothing while pending', () => {
  assert.equal(heldDone('pending', 'Kuni'), null);
  assert.equal(heldDone('allowed', 'Kuni'), 'Approved by Kuni');
  assert.equal(heldDone('allowed', null), 'Approved');
  assert.equal(heldDone('denied', 'Sarah'), 'Denied by Sarah');
  assert.equal(heldDone('denied', ''), 'Denied');
  assert.equal(heldDone('expired', 'Kuni'), 'Expired');
});

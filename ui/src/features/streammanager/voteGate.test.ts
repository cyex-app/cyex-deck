import assert from 'node:assert/strict';
import { test } from 'node:test';

import { firstEmpty, votesAllowed } from './voteGate.ts';

test('V110-N13 polls and predictions need Affiliate or Partner; unknown keeps the form', () => {
  assert.equal(votesAllowed({ affiliate: true, partner: false }), true);
  assert.equal(votesAllowed({ affiliate: false, partner: true }), true);
  assert.equal(votesAllowed({ affiliate: false, partner: false }), false);
  assert.equal(votesAllowed(null), null);
  assert.equal(votesAllowed(undefined), null);
});

test('V110-N13 focus goes to the first empty field', () => {
  assert.equal(firstEmpty(['', '', '']), 0);
  assert.equal(firstEmpty(['Best map?', '', '']), 1);
  assert.equal(firstEmpty(['Q', 'a', ' ']), 2);
  assert.equal(firstEmpty(['Q', 'a', 'b']), 0);
});

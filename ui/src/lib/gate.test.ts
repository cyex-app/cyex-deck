import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { AccountView } from '../ipc/bindings/AccountView';
import { FLAGS, lockFor, signInTarget } from './gate.ts';

const token = { source: 'device', expiresAt: null, state: 'valid' } as AccountView['helix'];
const account = (helix: boolean, gql: boolean) =>
  ({ userId: '1', helix: helix ? token : null, gql: gql ? token : null }) as AccountView;

test('a usable tool has no flag', () => {
  assert.equal(lockFor(true, 'gql'), null);
  assert.equal(lockFor(false, 'gql'), 'gql');
  assert.equal(FLAGS.gql.label, 'GQL only');
});

test('a flag click lands on the right sign-in', () => {
  assert.equal(signInTarget('helix', undefined), 'twitch');
  assert.equal(signInTarget('gql', undefined), 'code');
  // Signed in with Twitch: that account's GQL options.
  assert.equal(signInTarget('gql', account(true, false)), 'account');
  // Signed in with a code only: a code again, or Twitch's sign-in for the wallet.
  assert.equal(signInTarget('gql', account(false, true)), 'code');
  assert.equal(signInTarget('helix', account(false, true)), 'twitch');
});

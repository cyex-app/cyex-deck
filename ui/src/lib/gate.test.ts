import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { AccountView } from '../ipc/bindings/AccountView';
import { access, FLAGS, gqlUsable, helixUsable, lockFor, missingSignIn, NEEDS_TEXT, needsKind, signInTarget } from './gate.ts';
import { rawError } from './errors.ts';

const token = { source: 'device', expiresAt: null, state: 'valid' } as AccountView['helix'];
const account = (helix: boolean, gql: boolean) =>
  ({ userId: '1', helix: helix ? token : null, gql: gql ? token : null }) as AccountView;

test('a usable tool has no flag', () => {
  assert.equal(lockFor(true, 'gql'), null);
  assert.equal(lockFor(false, 'gql'), 'gql');
  assert.equal(FLAGS.gql.label, 'Extra Twitch features');
  assert.equal(FLAGS.helix.label, 'Twitch sign-in');
});

test('a flag click lands on the right sign-in', () => {
  // Nobody signed in: the matching way to add an account.
  assert.equal(signInTarget('helix', undefined), 'twitch');
  assert.equal(signInTarget('gql', undefined), 'code');
});

test('V111-47 with an account, the click lands on that account, whichever sign-in it lacks', () => {
  assert.equal(signInTarget('gql', account(true, false)), 'account');
  // A GQL-only account's Twitch sign-in reconnects it, it doesn't add another account.
  assert.equal(signInTarget('helix', account(false, true)), 'account');
  assert.equal(signInTarget('gql', account(false, true)), 'account');
});

test('V111-47 an error that says a sign-in is missing is told apart, word for word', () => {
  assert.equal(needsKind(NEEDS_TEXT.helix), 'helix');
  assert.equal(needsKind(NEEDS_TEXT.gql), 'gql');
  assert.equal(needsKind('Needs the Twitch sign-in.'), 'helix');
  assert.equal(needsKind('  Needs Extra Twitch features.  '), 'gql');
  assert.equal(needsKind('Voting needs the GQL sign-in (Settings → Accounts).'), null);
  assert.equal(needsKind('Needs Extra Twitch features. Try later.'), null);
  assert.equal(needsKind(''), null);
  // What a toast sees: a rejected string or a typed { kind, msg }.
  assert.equal(needsKind(rawError('Needs Extra Twitch features.')), 'gql');
  assert.equal(needsKind(rawError({ kind: 'message', msg: 'Needs the Twitch sign-in.' })), 'helix');
  assert.equal(needsKind(rawError(new Error('boom'))), null);
});

test('V111-47 a tool is open, locked behind a sign-in, or has no account', () => {
  assert.equal(access(true, true), 'open');
  assert.equal(access(true, false), 'locked');
  assert.equal(access(false, false), 'none');
  assert.equal(access(false, true), 'none');
});

test('V111-47 the sign-in offered to an account that cannot chat or clip', () => {
  const t = (state: string) => ({ source: 'device', expiresAt: null, state }) as AccountView['helix'];
  const a = (helix: string | null, gql: string | null) => ({ helix: helix ? t(helix) : null, gql: gql ? t(gql) : null }) as AccountView;
  // A Twitch sign-in without the scope: add the extra features.
  assert.equal(missingSignIn(a('valid', null)), 'gql');
  // A run-out token is signed in again, not replaced by the other kind.
  assert.equal(missingSignIn(a('valid', 'invalid')), 'gql');
  assert.equal(missingSignIn(a('invalid', 'valid')), 'helix');
  assert.equal(missingSignIn(a('invalid', null)), 'helix');
  // Extra features only: the Twitch sign-in.
  assert.equal(missingSignIn(a(null, 'valid')), 'helix');
  assert.equal(helixUsable(a('valid', null)), true);
  assert.equal(helixUsable(a('invalid', null)), false);
  assert.equal(helixUsable(a(null, null)), false);
});

test('V108-N01 only a GQL token in a usable state counts', () => {
  const g = (state: string | null) => ({ gql: state ? { source: 'code', expiresAt: null, state } : null }) as unknown as AccountView;
  assert.equal(gqlUsable(g('valid')), true);
  assert.equal(gqlUsable(g('offline')), true);
  assert.equal(gqlUsable(g('invalid')), false);
  assert.equal(gqlUsable(g(null)), false);
  assert.equal(gqlUsable(undefined), false);
});

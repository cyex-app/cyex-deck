import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { AccountView } from '../../ipc/bindings/AccountView';
import type { TokenView } from '../../ipc/bindings/TokenView';
import { accountName, missingScopesText, needsSignIn, tokenStatus } from './status.ts';

const token = (over: Partial<TokenView>): TokenView => ({ source: 'device', expiresAt: null, state: 'valid', ...over });

test('token status wording', () => {
  assert.equal(tokenStatus(token({})).text, "Signed in, doesn’t expire");
  assert.equal(tokenStatus(token({ expiresAt: 1_800_000_000 })).text, 'Signed in');
  assert.match(tokenStatus(token({ source: 'pasted', expiresAt: 1_800_000_000 })).text, /^Works until \d{4}\/\d{2}\/\d{2} \d{2}:\d{2}$/);
  assert.equal(tokenStatus(token({ state: 'invalid' })).tone, 'bad');
  assert.equal(tokenStatus(token({ state: 'offline' })).tone, 'warn');
});

test('sign-in needed only for dead tokens', () => {
  assert.equal(needsSignIn(null), false);
  assert.equal(needsSignIn(token({ state: 'offline' })), false);
  assert.equal(needsSignIn(token({ state: 'missing' })), true);
});

test('account names show the login when it differs', () => {
  const a = { displayName: 'Someone', login: 'someone' } as AccountView;
  assert.equal(accountName(a), 'Someone');
  assert.equal(accountName({ ...a, displayName: '誰か' }), '誰か (someone)');
});

test('missing scopes text', () => {
  assert.match(missingScopesText(1), /^1 permission missing/);
  assert.match(missingScopesText(3), /^3 permissions missing/);
});

import { test } from 'node:test';
import assert from 'node:assert/strict';

import { signInNeed } from './cyexSignIn.ts';

test('no Twitch sign-in: asks for it', () => {
  assert.equal(signInNeed({ state: 'noToken' })?.text, 'cyex needs the Twitch sign-in on this account.');
  assert.equal(signInNeed({ state: 'noToken' })?.button, 'Sign in');
});

test('refused: says so', () => {
  assert.equal(signInNeed({ state: 'refused' })?.text, 'cyex didn’t take this sign-in. Sign in again.');
});

test('other states need nothing', () => {
  assert.equal(signInNeed({ state: 'offline' }), null);
});

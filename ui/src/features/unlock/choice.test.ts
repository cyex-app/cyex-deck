import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { AccountView } from '../../ipc/bindings/AccountView';
import { asksConsent, doneOf, leftText, order, pickNeedsSignIn, plan, sharedProfile, twitchLeft } from './choice.ts';

const ok = { source: 'device', expiresAt: null, state: 'valid' } as AccountView['helix'];
const bad = { source: 'device', expiresAt: null, state: 'invalid' } as AccountView['helix'];
const none = { seventv: false, twitch: false };

const labels = (p: ReturnType<typeof plan>) => (p.kind === 'ask' ? p.options.map((o) => o.label) : p.kind);

test('each entry asks its own two or three', () => {
  assert.deepEqual(labels(plan('seventv', none, true)), ['Just 7TV', '7TV + subscriptions and wallet']);
  assert.deepEqual(labels(plan('twitch', none, false)), ['Just subscriptions and wallet', 'Also 7TV']);
  assert.deepEqual(labels(plan('settings', none, false)), ['7TV', 'Subscriptions and wallet', 'Both']);
});

test('options carry their parts and none is done while both are left', () => {
  const p = plan('settings', none, true);
  assert.equal(p.kind, 'ask');
  if (p.kind !== 'ask') return;
  assert.deepEqual(p.options.map((o) => o.parts), [['seventv'], ['twitch'], ['twitch', 'seventv']]);
  assert.ok(p.options.every((o) => !o.done));
});

test('one thing left skips the choice', () => {
  // 7TV menu, subscriptions done: just 7TV, consent given already.
  assert.deepEqual(plan('seventv', { seventv: false, twitch: true }, true), { kind: 'run', parts: ['seventv'], consent: false });
  // A wallet prompt with 7TV done.
  assert.deepEqual(plan('twitch', { seventv: true, twitch: false }, true), { kind: 'run', parts: ['twitch'], consent: false });
  // Settings, only 7TV left and never allowed: the consent still shows.
  assert.deepEqual(plan('settings', { seventv: false, twitch: true }, false), { kind: 'run', parts: ['seventv'], consent: true });
});

test('nothing to do when the entry’s own part or everything is done', () => {
  assert.deepEqual(plan('settings', { seventv: true, twitch: true }, true), { kind: 'none' });
  assert.deepEqual(plan('seventv', { seventv: true, twitch: false }, true), { kind: 'none' });
  assert.deepEqual(plan('twitch', { seventv: false, twitch: true }, true), { kind: 'none' });
});

test('subscriptions and wallet need both of Twitch’s sign-ins', () => {
  assert.deepEqual(twitchLeft({ helix: null, gql: null }), ['helix', 'gql']);
  assert.deepEqual(twitchLeft({ helix: ok, gql: null }), ['gql']);
  assert.deepEqual(twitchLeft({ helix: bad, gql: ok }), ['helix']);
  assert.deepEqual(twitchLeft({ helix: ok, gql: ok }), []);
});

test('7TV counts as done only signed in with consent', () => {
  const a = (consent: boolean, signedIn: boolean) => ({ helix: ok, gql: ok, seventv: { consent, signedIn, until: null } });
  assert.deepEqual(doneOf(a(true, true)), { seventv: true, twitch: true });
  assert.deepEqual(doneOf(a(false, true)), { seventv: false, twitch: true });
  assert.deepEqual(doneOf(a(true, false)), { seventv: false, twitch: true });
});

test('a both option without consent shows it, and its click gives it', () => {
  assert.equal(asksConsent(['twitch', 'seventv'], false), true);
  assert.equal(asksConsent(['seventv'], false), true);
  assert.equal(asksConsent(['seventv'], true), false);
  assert.equal(asksConsent(['twitch'], false), false);
});

test('only a both choice borrows the Twitch profile', () => {
  assert.equal(sharedProfile(['twitch', 'seventv']), true);
  assert.equal(sharedProfile(['seventv', 'twitch']), true);
  assert.equal(sharedProfile(['seventv']), false);
  assert.equal(sharedProfile(['twitch']), false);
  // Each entry's both option, as the dialog passes it.
  for (const e of ['seventv', 'twitch', 'settings'] as const) {
    const p = plan(e, { seventv: false, twitch: false }, false);
    if (p.kind !== 'ask') throw new Error('asks');
    assert.deepEqual(p.options.map((o) => sharedProfile(o.parts)), e === 'settings' ? [false, false, true] : [false, true]);
  }
});

test('Twitch runs first', () => {
  assert.deepEqual(order(['seventv', 'twitch']), ['twitch', 'seventv']);
  assert.deepEqual(order(['seventv']), ['seventv']);
});

test('a half-done run says what’s left', () => {
  assert.equal(leftText(['twitch'], ['seventv']), 'Subscriptions and wallet is set. 7TV didn’t finish. Try again any time.');
  assert.equal(leftText([], ['twitch']), 'Subscriptions and wallet didn’t finish. Try again any time.');
});

test('a pick asks for sign-in only when allowed and signed out or expired', () => {
  assert.equal(pickNeedsSignIn('signedOut', true), true);
  assert.equal(pickNeedsSignIn('expired', true), true);
  assert.equal(pickNeedsSignIn('ready', true), false);
  assert.equal(pickNeedsSignIn('signedOut', false), false);
  assert.equal(pickNeedsSignIn('ready', true, { type: 'signedOut' }), true);
  assert.equal(pickNeedsSignIn('ready', true, { type: 'signInAgain' }), true);
  assert.equal(pickNeedsSignIn('ready', true, { type: 'down' }), false);
  assert.equal(pickNeedsSignIn('ready', true, { type: 'noConsent' }), false);
});

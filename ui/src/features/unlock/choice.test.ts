import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { AccountView } from '../../ipc/bindings/AccountView';
import { asksConsent, chosen, doneOf, leftText, order, pickNeedsSignIn, plan, sevenTvShared, twitchLeft } from './choice.ts';

const ok = { source: 'device', expiresAt: null, state: 'valid' } as AccountView['helix'];
const bad = { source: 'device', expiresAt: null, state: 'invalid' } as AccountView['helix'];
const none = { seventv: false, twitch: false };

const ticks = (p: ReturnType<typeof plan>) =>
  p.kind === 'ask' ? p.items.map((i) => `${i.part}:${i.ticked ? 'on' : 'off'}${i.done ? ':done' : ''}`) : p.kind;

test('each entry pre-ticks what it implies', () => {
  assert.deepEqual(ticks(plan('seventv', none, true)), ['seventv:on', 'twitch:off']);
  assert.deepEqual(ticks(plan('twitch', none, false)), ['seventv:off', 'twitch:on']);
  assert.deepEqual(ticks(plan('settings', none, false)), ['seventv:off', 'twitch:off']);
});

test('the two tickboxes are named', () => {
  const p = plan('settings', none, true);
  assert.equal(p.kind, 'ask');
  if (p.kind !== 'ask') return;
  assert.deepEqual(p.items.map((i) => i.part), ['seventv', 'twitch']);
  assert.match(p.items[0].label, /^7TV — /);
  assert.match(p.items[1].label, /^Subscriptions and wallet — /);
});

test('chosen runs only ticked parts that are not done', () => {
  const p = plan('settings', none, true);
  if (p.kind !== 'ask') throw new Error('asks');
  assert.deepEqual(chosen(p.items, { seventv: false, twitch: false }), []);
  assert.deepEqual(chosen(p.items, { seventv: true, twitch: true }), ['seventv', 'twitch']);
  const q = { ...p, items: p.items.map((i) => (i.part === 'twitch' ? { ...i, done: true, ticked: true } : i)) };
  assert.deepEqual(chosen(q.items, { seventv: true, twitch: true }), ['seventv']);
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

test('7TV follows Open in', () => {
  assert.equal(sevenTvShared('browser'), true);
  assert.equal(sevenTvShared('app'), false);
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

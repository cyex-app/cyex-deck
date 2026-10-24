import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { AccountView } from '../../ipc/bindings/AccountView';
import { asksSevenTv, linksOf } from './alsoSevenTv.ts';

const t = { source: 'device', expiresAt: null, state: 'valid' } as AccountView['helix'];

test('links: tokens (expired too) and Twitch pages; no account is null', () => {
  assert.deepEqual(linksOf({ helix: t, gql: null }, false), ['helix']);
  assert.deepEqual(linksOf({ helix: t, gql: t }, true), ['helix', 'gql', 'pages']);
  assert.deepEqual(linksOf({ helix: null, gql: null }, false), []);
  assert.equal(linksOf(undefined, true), null);
});

test('asks when the code sign-in meets Sign in with Twitch or Twitch pages, either order', () => {
  assert.equal(asksSevenTv('helix', ['gql'], false, false), true);
  assert.equal(asksSevenTv('pages', ['gql'], false, false), true);
  assert.equal(asksSevenTv('gql', ['helix'], false, false), true);
  assert.equal(asksSevenTv('gql', ['pages'], false, false), true);
});

test('no ask for a new account, a lone link, Sign in with Twitch plus pages, or a pair already there', () => {
  assert.equal(asksSevenTv('helix', null, false, false), false);
  assert.equal(asksSevenTv('gql', [], false, false), false);
  assert.equal(asksSevenTv('pages', ['helix'], false, false), false);
  assert.equal(asksSevenTv('helix', ['helix', 'gql'], false, false), false);
  assert.equal(asksSevenTv('pages', ['helix', 'gql'], false, false), false);
});

test('no ask with 7TV connected or after Not now', () => {
  assert.equal(asksSevenTv('helix', ['gql'], true, false), false);
  assert.equal(asksSevenTv('helix', ['gql'], false, true), false);
});

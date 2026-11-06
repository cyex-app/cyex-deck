import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { BadgeSets } from '../../ipc/bindings/BadgeSets.ts';
import { BADGE_SETS, badgeOn, setOf } from './badgeSets.ts';

const off: BadgeSets = { ffzap: false, chatsen: false, dankchat: false, homies: false };
const providers = { twitch: true, seventv: true, bttv: false, ffz: true };

test('ids tell their set', () => {
  assert.equal(setOf('ffzap:4867723'), 'ffzap');
  assert.equal(setOf('homies:u59842770'), 'homies');
  assert.equal(setOf('staff'), null);
  assert.equal(setOf('other:x'), null);
  assert.equal(setOf(':x'), null);
  for (const s of BADGE_SETS) assert.equal(setOf(`${s.key}:1`), s.key);
});

test('an extra badge follows its own switch, not Twitch badges', () => {
  const b = { provider: 'twitch', id: 'ffzap:founder' };
  assert.equal(badgeOn(b, providers, off), false);
  assert.equal(badgeOn(b, providers, { ...off, ffzap: true }), true);
  assert.equal(badgeOn(b, { ...providers, twitch: false }, { ...off, ffzap: true }), true);
});

test('V109-B02 cyex badges follow the cyex switch', () => {
  const b = { provider: 'twitch', id: 'cyex:founder-37' };
  assert.equal(badgeOn(b, { ...providers, cyex: true, twitch: false }, off), true);
  assert.equal(badgeOn(b, { ...providers, cyex: false }, off), false);
  assert.equal(badgeOn(b, providers, off), false, 'no switch: off');
});

test('the built-in providers keep their switches', () => {
  assert.equal(badgeOn({ provider: 'seventv', id: 'a' }, providers, off), true);
  assert.equal(badgeOn({ provider: 'bttv', id: 'pro' }, providers, off), false);
  assert.equal(badgeOn({ provider: 'ffz', id: '2' }, providers, off), true);
});

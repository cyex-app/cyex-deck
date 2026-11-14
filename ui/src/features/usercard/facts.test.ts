import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { IvrUser } from '../../ipc/bindings/IvrUser.ts';
import type { UsercardShow } from '../../ipc/bindings/UsercardShow.ts';
import { setAppClock } from '../../lib/format.ts';
import { chatHex, giftText, lastLiveText, lookups, roleSummary, sinceText } from './facts.ts';

const ALL: UsercardShow = {
  created: true,
  followers: true,
  sub: true,
  gifter: true,
  followAge: true,
  names: true,
  lastLive: true,
  chatColor: true,
  chips: true,
  verifiedBot: true,
  badges: true,
  paint: true,
  seventvProfile: true,
  userId: true,
  otherModCounts: true,
  roles: true,
  follow: true,
  modTabs: true,
};
const ivr = (o: Partial<IvrUser> = {}): IvrUser => ({
  lastLive: null,
  lastTitle: null,
  live: false,
  verifiedBot: false,
  chatColor: null,
  ...o,
});

test('V111-13 off means no lookup', () => {
  assert.deepEqual(lookups(ALL), {
    ivr: true,
    sub: true,
    followAge: true,
    names: true,
    roles: true,
    otherMods: true,
    seventv: true,
  });
  const off = Object.fromEntries(Object.keys(ALL).map((k) => [k, false])) as unknown as UsercardShow;
  assert.ok(Object.values(lookups(off)).every((v) => !v), 'everything off asks nobody');
  assert.equal(lookups({ ...ALL, lastLive: false }).ivr, true, 'the bot chip still asks');
  assert.equal(lookups({ ...ALL, lastLive: false, verifiedBot: false }).ivr, false);
  assert.equal(lookups({ ...off, chatColor: true }).ivr, false, 'the colour comes from the profile');
  assert.equal(lookups({ ...off, gifter: true }).sub, false, 'the gifter comes with the sub status');
});

test('V111-13 last live, in the app formats', () => {
  setAppClock('h24');
  const at = new Date(2026, 10, 2, 19, 40).toISOString();
  assert.equal(lastLiveText(ivr({ lastLive: at })), 'Last live 2026/11/02, 19:40');
  assert.equal(lastLiveText(ivr({ lastLive: at, live: true })), 'Live since 19:40');
  assert.equal(lastLiveText(ivr()), null);
  assert.equal(lastLiveText(ivr({ lastLive: 'soon' })), null);
  assert.equal(lastLiveText(null), null);
});

test('V111-13 who gifted it', () => {
  const g = { gift: true, gifter: 'Giver', giftAnonymous: false };
  assert.equal(giftText(g, true), 'gifted by Giver');
  assert.equal(giftText(g, false), 'gifted');
  assert.equal(giftText({ gift: true, gifter: null, giftAnonymous: true }, true), 'gifted anonymously');
  assert.equal(giftText({ gift: true, gifter: null, giftAnonymous: false }, true), 'gifted');
  assert.equal(giftText({ gift: false, gifter: null, giftAnonymous: false }, true), '');
});

test('V111-13 the chat colour: the profile first, then ivr, hex only', () => {
  assert.equal(chatHex('#8a2be2', ivr({ chatColor: '#0029CC' })), '#8A2BE2');
  assert.equal(chatHex(null, ivr({ chatColor: '#0029cc' })), '#0029CC');
  assert.equal(chatHex('red', null), null);
  assert.equal(chatHex(null, null), null);
});

test('V111-13 roles summary and dates', () => {
  assert.deepEqual(
    roleSummary({ mods: 1200, vips: 0, founders: 2, artists: 0 }).map((r) => r.text),
    ['Mod in 1.200', 'Founder in 2'],
  );
  assert.deepEqual(roleSummary({ mods: 0, vips: 0, founders: 0, artists: 0 }), []);
  assert.equal(sinceText(new Date(2020, 7, 11, 12).toISOString()), 'since 2020/08/11');
  assert.equal(sinceText(null), '');
  assert.equal(sinceText('x'), '');
});

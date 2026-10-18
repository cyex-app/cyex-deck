import assert from 'node:assert/strict';
import { test } from 'node:test';

import { cancelEmoteMenu, emoteEditMenu, within } from './menu.ts';

const ready = {
  state: 'ready',
  ownName: 'KEKW',
  add: null,
  here: { setId: 's', channel: 'kuni', alias: 'KEKW' },
} as never;
const h = { add: () => {}, rename: () => {}, remove: () => {} };
const emote = { id: 'e', name: 'KEKW', provider: 'seventv' };
const later = <T>(v: T, ms: number) => new Promise<T>((r) => setTimeout(() => r(v), ms));

test('a stub that answers after the wait gives null options', async () => {
  assert.equal(await within(later(ready, 40), 5), null);
  assert.equal(await within(later(ready, 1), 40), ready);
});

test('entries when 7TV answers in time', async () => {
  const r = await emoteEditMenu('me', 'kuni', emote, h, 50, { target: new EventTarget(), load: async () => ready });
  assert.ok(r && r.length > 0);
});

test('plain menu when 7TV is too slow', async () => {
  const r = await emoteEditMenu('me', 'kuni', emote, h, 5, { target: new EventTarget(), load: () => later(ready, 40) });
  assert.deepEqual(r, []);
});

test('the newest click wins', async () => {
  const t = new EventTarget();
  const slow = emoteEditMenu('me', 'kuni', emote, h, 50, { target: t, load: () => later(ready, 10) });
  const fast = emoteEditMenu('me', 'kuni', emote, h, 50, { target: t, load: async () => ready });
  assert.equal(await slow, null);
  assert.ok(await fast);
});

test('another kind of right-click drops it too', async () => {
  const p = emoteEditMenu('me', 'kuni', emote, h, 50, { target: new EventTarget(), load: () => later(ready, 10) });
  cancelEmoteMenu();
  assert.equal(await p, null);
});

test('a dismiss during the wait gives null', async () => {
  const t = new EventTarget();
  const p = emoteEditMenu('me', 'kuni', emote, h, 50, { target: t, load: () => later(ready, 10) });
  t.dispatchEvent(new Event('pointerdown'));
  assert.equal(await p, null);
});

test('not 7TV or no account: nothing to add', async () => {
  assert.deepEqual(await emoteEditMenu('me', 'kuni', { ...emote, provider: 'bttv' }, h), []);
  assert.deepEqual(await emoteEditMenu(null, 'kuni', emote, h), []);
});

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

// V112-17 cyex.tv emotes get "Add to cyex.tv", only where the account may edit.
const tvEmote = { id: '01E.key', name: 'Hello', provider: 'cyextv' };
const said: [string, boolean][] = [];
const tv = (o: { canEdit: boolean; note: string | null }, add = async () => 'Added.') => ({
  options: async () => o,
  add,
  said: (line: string, ok: boolean) => said.push([line, ok]),
});

test('cyex.tv emote: add entry when the channel is editable', async () => {
  const r = await emoteEditMenu('me', 'kuni', tvEmote, h, 50, { target: new EventTarget(), ready: true, tv: tv({ canEdit: true, note: null }) });
  assert.ok(r);
  const item = r.find((e) => typeof e === 'object' && 'label' in e);
  assert.ok(item && typeof item === 'object' && 'label' in item);
  assert.equal(item.label, 'Add to cyex.tv');
  assert.ok(!item.disabled);
  item.run?.();
  await later(0, 5);
  assert.deepEqual(said.at(-1), ['Added.', true]);
});

test('cyex.tv emote: nothing without edit rights, greyed with a reason', async () => {
  const none = await emoteEditMenu('me', 'kuni', tvEmote, h, 50, { target: new EventTarget(), ready: true, tv: tv({ canEdit: false, note: null }) });
  assert.deepEqual(none, []);
  const grey = await emoteEditMenu('me', 'kuni', tvEmote, h, 50, {
    target: new EventTarget(),
    ready: true,
    tv: tv({ canEdit: true, note: 'Already in kuni’s cyex.tv emotes.' }),
  });
  const item = grey?.find((e) => typeof e === 'object' && 'label' in e);
  assert.ok(item && typeof item === 'object' && 'disabled' in item && item.disabled);
});

test('cyex.tv emote: a failed add says why', async () => {
  const r = await emoteEditMenu('me', 'kuni', tvEmote, h, 50, {
    target: new EventTarget(),
    ready: true,
    tv: tv({ canEdit: true, note: null }, async () => Promise.reject('You can’t edit kuni’s cyex.tv emotes.')),
  });
  const item = r?.find((e) => typeof e === 'object' && 'label' in e);
  assert.ok(item && typeof item === 'object' && 'run' in item);
  item.run?.();
  await later(0, 5);
  assert.deepEqual(said.at(-1), ['You can’t edit kuni’s cyex.tv emotes.', false]);
});

test('V112-23 cyex.tv coming soon: no "Add to cyex.tv", nothing asked', async () => {
  let asked = 0;
  const r = await emoteEditMenu('me', 'kuni', tvEmote, h, 50, {
    target: new EventTarget(),
    ready: false,
    tv: {
      options: async () => {
        asked++;
        return { canEdit: true, note: null };
      },
      add: async () => 'added',
      said: () => {},
    },
  });
  assert.deepEqual(r, []);
  assert.equal(asked, 0);
});

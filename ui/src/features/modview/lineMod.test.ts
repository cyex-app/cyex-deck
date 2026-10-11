import assert from 'node:assert/strict';
import { test } from 'node:test';

import { modButtonsFor, type LineMod } from './lineMod.ts';

const mod: LineMod = { me: '1', broadcaster: false, inline: true, timeout: 60 };
const user = (id: string, login: string, ...sets: string[]) => ({ id, login, badges: sets.map((set) => ({ set })) });

test('mod mode puts buttons on viewers only (V101-33)', () => {
  assert.equal(modButtonsFor(mod, user('2', 'viewer', 'subscriber'), 'chan'), true);
  assert.equal(modButtonsFor(mod, user('1', 'me'), 'chan'), false, 'own lines');
  assert.equal(modButtonsFor(mod, user('3', 'chan', 'broadcaster'), 'chan'), false, 'the broadcaster');
  assert.equal(modButtonsFor(mod, user('4', 'othermod', 'moderator'), 'chan'), false, 'another mod');
  assert.equal(modButtonsFor(mod, user('5', 'lead', 'lead_moderator'), 'chan'), false, 'a lead mod');
});

test('the broadcaster gets them on mods too', () => {
  const owner = { ...mod, broadcaster: true };
  assert.equal(modButtonsFor(owner, user('4', 'othermod', 'moderator'), 'chan'), true);
});

test('nothing with mod mode off or without a mod role', () => {
  assert.equal(modButtonsFor({ ...mod, inline: false }, user('2', 'viewer'), 'chan'), false);
  assert.equal(modButtonsFor(null, user('2', 'viewer'), 'chan'), false);
  assert.equal(modButtonsFor(mod, null, 'chan'), false);
});

import assert from 'node:assert/strict';
import { test } from 'node:test';

import { cantDelete, DELETE_WINDOW_MS, type DeleteTarget } from './deleteRule.ts';

const now = 1_800_000_000_000;
const viewer: DeleteTarget = { id: '9', login: 'someone', role: null, ts: now - 1000 };
const mod: DeleteTarget = { id: '8', login: 'amod', role: 'moderator', ts: now - 1000 };
const lead: DeleteTarget = { id: '7', login: 'alead', role: 'leadModerator' };
const owner: DeleteTarget = { id: '1', login: 'Chan', role: 'broadcaster' };

test('V107 a mod deletes viewers and their own lines only', () => {
  assert.equal(cantDelete('moderator', '8', viewer, 'chan', now), null);
  assert.equal(cantDelete('moderator', '8', mod, 'chan', now), null);
  assert.match(cantDelete('moderator', '5', mod, 'chan', now) ?? '', /other mods/);
  assert.match(cantDelete('leadModerator', '5', lead, 'chan', now) ?? '', /other mods/);
  assert.match(cantDelete('leadModerator', '5', owner, 'chan', now) ?? '', /broadcaster/);
});

test('V107 the broadcaster is known by login when the badges are missing', () => {
  assert.match(cantDelete('moderator', '8', { ...owner, role: null }, 'chan', now) ?? '', /broadcaster/);
});

test('V107 the broadcaster deletes anything recent, their own lines too', () => {
  for (const t of [viewer, mod, lead, owner]) assert.equal(cantDelete('broadcaster', '1', t, 'chan', now), null);
});

test('V107 lines over 6 hours old and non-mods are refused', () => {
  const old = { ...viewer, ts: now - DELETE_WINDOW_MS - 1 };
  assert.match(cantDelete('broadcaster', '1', old, 'chan', now) ?? '', /6 hours/);
  assert.match(cantDelete(null, '9', viewer, 'chan', now) ?? '', /Only mods/);
});

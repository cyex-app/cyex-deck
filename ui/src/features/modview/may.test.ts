import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

import { may, whose } from './may.ts';

// V107-R14 Rust's answers per role, written by cargo test (crates/moderation/src/may.rs).
const caps = JSON.parse(readFileSync(new URL('../../ipc/bindings/caps.fixture.json', import.meta.url), 'utf8'));
const now = 1_800_000_000_000;
const SIX_H = 6 * 60 * 60 * 1000;

test('lines sort like Rust sorts them', () => {
  const s = (id: string, login: string, ...sets: string[]) => ({ id, login, badges: sets.map((set) => ({ set })) });
  assert.equal(whose(s('1', 'Chan'), '9', 'chan'), 'broadcaster', 'by login, badges missing');
  assert.equal(whose(s('1', 'x', 'broadcaster'), '9', 'chan'), 'broadcaster');
  assert.equal(whose(s('1', 'chan'), '1', 'chan'), 'broadcaster', 'the broadcaster before own');
  assert.equal(whose(s('8', 'amod', 'moderator'), '8', 'chan'), 'own');
  assert.equal(whose(s('7', 'alead', 'lead_moderator'), '8', 'chan'), 'mod');
  assert.equal(whose(s('', 'x', 'moderator'), null, 'chan'), 'mod', 'no id is never own');
  assert.equal(whose({ id: '', login: 'viewer' }, '8', 'chan'), 'anyone');
});

test('the cell follows the message age', () => {
  const m = caps.moderator;
  assert.equal(may(m, 'delete', 'anyone', now - 1000, now).why, null);
  assert.equal(may(m, 'delete', 'anyone', null, now).why, null, 'unknown age counts as fresh');
  assert.match(may(m, 'delete', 'anyone', now - SIX_H - 1, now).why ?? '', /6 hours/);
  assert.equal(may(m, 'delete', 'anyone', now - SIX_H, now).why, null, 'exactly 6 hours still goes');
  assert.match(may(m, 'delete', 'mod', now, now).why ?? '', /other mods/);
  assert.equal(may(caps.broadcaster, 'delete', 'broadcaster', now, now).offered, true);
  assert.equal(may(caps.broadcaster, 'delete', 'broadcaster', now - SIX_H - 1, now).offered, false);
  assert.equal(may(caps.leadModerator, 'user', 'broadcaster', now, now).offered, false);
});

test('not a mod here: nothing offered', () => {
  assert.deepEqual(may(null, 'pin', 'anyone'), { offered: false, why: null });
});

import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { Caps } from '../../ipc/bindings/Caps';
import { activeModes, canSetModes, FOLLOW_STEPS, modeCommand, SLOW_STEPS, withChange } from './roomModes.ts';

const off = { slow: 0, followersOnly: null, subsOnly: false, emoteOnly: false, unique: false };

test('the chip lists what is on, in the header order', () => {
  assert.deepEqual(activeModes(off), []);
  assert.deepEqual(
    activeModes({ slow: 30, followersOnly: 0, subsOnly: true, emoteOnly: true, unique: true }).map((m) => m.label),
    ['Slow 30s', 'Followers', 'Sub', 'Emotes', 'Unique'],
  );
  assert.equal(activeModes({ ...off, followersOnly: 60 })[0].label, 'Followers 1h');
});

test('only an account with mod caps changes modes', () => {
  assert.equal(canSetModes(null), false);
  assert.equal(canSetModes(undefined), false);
  assert.equal(canSetModes({} as Caps), true);
});

test('lengths: slow in seconds, followers in minutes up to 3 months', () => {
  assert.deepEqual(SLOW_STEPS.map((s) => s.secs), [3, 5, 10, 30, 60, 120, 300]);
  assert.deepEqual(FOLLOW_STEPS.map((s) => s.mins), [0, 10, 30, 60, 1440, 10_080, 43_200, 129_600]);
});

test('each change sends the command the chat already knows', () => {
  assert.equal(modeCommand({ kind: 'emote', on: true }), '/emoteonly');
  assert.equal(modeCommand({ kind: 'emote', on: false }), '/emoteonlyoff');
  assert.equal(modeCommand({ kind: 'subs', on: true }), '/subscribers');
  assert.equal(modeCommand({ kind: 'subs', on: false }), '/subscribersoff');
  assert.equal(modeCommand({ kind: 'unique', on: true }), '/uniquechat');
  assert.equal(modeCommand({ kind: 'unique', on: false }), '/uniquechatoff');
  assert.equal(modeCommand({ kind: 'slow', secs: 5 }), '/slow 5');
  assert.equal(modeCommand({ kind: 'slow', secs: null }), '/slowoff');
  assert.equal(modeCommand({ kind: 'followers', mins: 0 }), '/followers');
  assert.equal(modeCommand({ kind: 'followers', mins: 10_080 }), '/followers 10080');
  assert.equal(modeCommand({ kind: 'followers', mins: null }), '/followersoff');
});

test('the optimistic state applies the change and leaves the rest', () => {
  assert.deepEqual(withChange(off, { kind: 'slow', secs: 10 }), { ...off, slow: 10 });
  assert.deepEqual(withChange({ ...off, slow: 10 }, { kind: 'slow', secs: null }), off);
  assert.deepEqual(withChange(off, { kind: 'followers', mins: 0 }), { ...off, followersOnly: 0 });
  assert.deepEqual(withChange({ ...off, followersOnly: 0 }, { kind: 'followers', mins: null }), off);
  assert.equal(withChange(off, { kind: 'subs', on: true }).subsOnly, true);
});

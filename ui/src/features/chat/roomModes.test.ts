import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { Caps } from '../../ipc/bindings/Caps';
import { activeModes, canSetModes, followText, lastModes, modeCommand, parseFollowers, parseSlow, rememberMode, slowText, withChange } from './roomModes.ts';

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

const val = (r: { value: number } | { error: string }) => ('value' in r ? r.value : r.error);
const SLOW_ERR = 'Slow mode goes from 3 seconds to 2 minutes.';
const FOL_ERR = 'Followers only goes from 0 minutes to 3 months.';

test('V112-29 slow mode: units, bare seconds, 3 s to 2 min', () => {
  assert.equal(val(parseSlow('30')), 30);
  assert.equal(val(parseSlow('30s')), 30);
  assert.equal(val(parseSlow(' 2M ')), 120);
  assert.equal(val(parseSlow('1 m')), 60);
  assert.equal(val(parseSlow('3')), 3);
  assert.equal(val(parseSlow('120')), 120);
  for (const bad of ['2', '0', '121', '3m', '1h', '1d', '1w', '3mo', '-5', '1.5m', '2.5', '', ' ', 'abc', '5x', 's', '10 20', '99999999999']) {
    assert.equal(val(parseSlow(bad)), SLOW_ERR, bad);
  }
});

test('V112-29 followers only: units, bare minutes, 0 to 3 months', () => {
  assert.equal(val(parseFollowers('0')), 0);
  assert.equal(val(parseFollowers('0s')), 0);
  assert.equal(val(parseFollowers('30')), 30);
  assert.equal(val(parseFollowers('30m')), 30);
  assert.equal(val(parseFollowers('120s')), 2);
  assert.equal(val(parseFollowers('1h')), 60);
  assert.equal(val(parseFollowers('1d')), 1440);
  assert.equal(val(parseFollowers('1w')), 10_080);
  assert.equal(val(parseFollowers('1mo')), 43_200);
  assert.equal(val(parseFollowers('3mo')), 129_600);
  assert.equal(val(parseFollowers('129600')), 129_600);
  for (const bad of ['129601', '4mo', '13w', '-1', '1.5h', '0.5', '30s', '', 'junk', '1y', '99999999999']) {
    assert.equal(val(parseFollowers(bad)), FOL_ERR, bad);
  }
});

test('V112-29 the field starts with the last length used, or the default', () => {
  assert.deepEqual(lastModes(), { slow: 30, followers: 0 });
  assert.equal(slowText(30), '30s');
  assert.equal(slowText(120), '2m');
  assert.equal(followText(0), '0');
  assert.equal(followText(60), '1h');
  assert.equal(followText(129_600), '3mo');
  rememberMode({ kind: 'slow', secs: 10 });
  rememberMode({ kind: 'followers', mins: 1440 });
  rememberMode({ kind: 'slow', secs: null });
  rememberMode({ kind: 'followers', mins: null });
  assert.deepEqual(lastModes(), { slow: 10, followers: 1440 });
  rememberMode({ kind: 'slow', secs: 30 });
  rememberMode({ kind: 'followers', mins: 0 });
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

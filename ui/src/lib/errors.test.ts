import assert from 'node:assert/strict';
import { test } from 'node:test';

import { errorKind, errorText, sentence } from './errors.ts';

test('Twitch client errors become sentences', () => {
  assert.equal(errorText('couldn’t reach Twitch'), 'Couldn’t reach Twitch. Check your connection.');
  assert.equal(errorText(new TypeError('Failed to fetch')), 'Couldn’t reach Twitch. Check your connection.');
  assert.equal(errorText('Twitch said: invalid oauth token (401)'), 'Your sign-in ran out. Sign in again.');
  assert.equal(errorText('Twitch said: nope (403)'), 'Twitch won’t allow that.');
  assert.equal(errorText('Twitch said: Too Many Requests (429)'), 'Too many tries. Wait a moment.');
  assert.equal(errorText('Twitch said: (502)'), 'Twitch didn’t answer. Try again in a bit.');
  assert.equal(errorText('Twitch said: title is too long (400)'), 'Twitch said: title is too long.');
  assert.equal(errorText('unexpected answer from Twitch'), 'Couldn’t read Twitch’s answer.');
});

test('GQL refusals hide the raw code', () => {
  assert.equal(errorText('Twitch said no: NOT_ENOUGH_POINTS'), 'Not enough channel points.');
  assert.equal(errorText('Twitch said no: SOME_GQL_CODE'), 'Twitch said no.');
  assert.equal(errorText('Twitch said no (WEIRD_CODE).'), 'Twitch said no.');
  assert.equal(errorKind('Twitch said no: X'), 'refused');
});

test('internal errors use the action, or a plain fallback', () => {
  const tauri = 'invalid args `userId` for command `set_default`: missing field';
  assert.equal(errorText(tauri, 'switch accounts'), 'Couldn’t switch accounts.');
  assert.equal(errorText(tauri), 'Something went wrong. Try again.');
  assert.equal(errorText(undefined, 'open that'), 'Couldn’t open that.');
  assert.equal(errorText({}), 'Something went wrong. Try again.');
});

test('the app’s own messages stay, tidied', () => {
  assert.equal(errorText('Sign in with Twitch first.'), 'Sign in with Twitch first.');
  assert.equal(errorText('that isn’t a cyex settings file'), 'That isn’t a cyex settings file.');
  assert.equal(errorText(new Error("couldn't save")), 'Couldn’t save.');
});

test('a typed error from Rust is read by kind and msg', () => {
  assert.equal(errorText({ kind: 'rate', msg: 'whatever' }), 'Too many tries. Wait a moment.');
  assert.equal(errorText({ kind: 'message', msg: 'No such recording.' }), 'No such recording.');
  assert.equal(errorKind({ kind: 'bogus', msg: 'couldn’t reach Twitch' }), 'network');
});

test('sentence', () => {
  assert.equal(sentence('done'), 'Done.');
  assert.equal(sentence('Done!'), 'Done!');
  assert.equal(sentence('  '), '');
  // V106-31 A finished line from Rust that starts with a login keeps its case.
  assert.equal(sentence('mockme has no 7TV emote called nope.'), 'mockme has no 7TV emote called nope.');
  // K10 Library text ending in ")" gets a capital, no full stop.
  assert.equal(sentence('access is denied. (os error 5)'), 'Access is denied. (os error 5)');
  assert.equal(sentence('error sending request for url (http://x)'), 'Error sending request for url (http://x)');
  assert.equal(errorText('xQc already has an emote named peepoSad.'), 'xQc already has an emote named peepoSad.');
});

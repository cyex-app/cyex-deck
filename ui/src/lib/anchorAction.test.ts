import assert from 'node:assert/strict';
import { test } from 'node:test';

import { anchorAction } from './anchorAction.ts';

const O = 'http://tauri.localhost';

test('V110-F83 hash and same-origin links stay untouched', () => {
  assert.equal(anchorAction(O + '/#', O, false), 'ignore');
  assert.equal(anchorAction(O + '/#top', O, false), 'ignore');
});

test('V110-F83 outside http(s) links are stopped and opened', () => {
  assert.equal(anchorAction('https://twitch.tv/x', O, false), 'open');
  assert.equal(anchorAction('http://example.com/', O, false), 'open');
});

test('V110-F83 javascript, data and other schemes are stopped, not opened', () => {
  assert.equal(anchorAction('javascript:alert(1)', O, false), 'stop');
  assert.equal(anchorAction('data:text/html,hi', O, false), 'stop');
  assert.equal(anchorAction('mailto:a@b.c', O, false), 'stop');
  assert.equal(anchorAction('not a url', O, false), 'stop');
});

test('V110-F83 an already handled click is left alone', () => {
  assert.equal(anchorAction('https://twitch.tv/x', O, true), 'ignore');
});

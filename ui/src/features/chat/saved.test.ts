import assert from 'node:assert/strict';
import { test } from 'node:test';

import { saved, useSaved } from './saved.ts';

test('saved pictures only while a recording is in use', () => {
  assert.equal(saved('emote:twitch:25'), undefined);
  useSaved('abc_1', { 'emote:twitch:25': 'emote_twitch_25.png' });
  assert.equal(saved('emote:twitch:25'), 'http://cyexhls.localhost/rec/abc_1/emote_twitch_25.png');
  assert.equal(saved('badge:vip/1'), undefined);
});

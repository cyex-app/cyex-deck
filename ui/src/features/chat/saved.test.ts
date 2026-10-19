import assert from 'node:assert/strict';
import { test } from 'node:test';

import { dropSaved, saved, useSaved } from './saved.ts';

test('saved pictures only while a recording is in use', () => {
  assert.equal(saved('emote:twitch:25'), undefined);
  useSaved('abc_1', { 'emote:twitch:25': 'emote_twitch_25.png' });
  assert.equal(saved('emote:twitch:25'), 'http://cyexhls.localhost/rec/abc_1/emote_twitch_25.png');
  assert.equal(saved('badge:vip/1'), undefined);
});

test('a closed recording forgets its pictures, a newer one keeps its own', () => {
  useSaved('a_1', { 'gif:1': 'g1.gif' });
  useSaved('b_2', { 'gif:2': 'g2.gif' });
  dropSaved('a_1');
  assert.equal(saved('gif:2'), 'http://cyexhls.localhost/rec/b_2/g2.gif');
  dropSaved('b_2');
  assert.equal(saved('gif:2'), undefined);
});

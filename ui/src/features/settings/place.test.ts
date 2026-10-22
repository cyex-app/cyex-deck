import assert from 'node:assert/strict';
import { test } from 'node:test';

import { settingsPlace } from './place.svelte.ts';

test('V107-Mo the place keeps the search and open parts per page, and resets', () => {
  settingsPlace.reset();
  assert.equal(settingsPlace.query, '');
  assert.equal(settingsPlace.open('notifications', 'channels'), false);
  assert.equal(settingsPlace.open('notifications', 'channels', true), true);
  settingsPlace.query = 'font';
  settingsPlace.setOpen('notifications', 'channels', true);
  settingsPlace.setOpen('notifications', 'group:Chat', true);
  assert.equal(settingsPlace.query, 'font');
  assert.equal(settingsPlace.open('notifications', 'channels'), true);
  assert.equal(settingsPlace.open('notifications', 'group:Chat'), true);
  assert.equal(settingsPlace.open('notifications', 'group:Other'), false);
  settingsPlace.reset();
  assert.equal(settingsPlace.query, '');
  assert.equal(settingsPlace.open('notifications', 'channels', false), false);
});

test('V107-Mo another page drops the search; the same page keeps it', () => {
  settingsPlace.reset();
  assert.equal(settingsPlace.arrive('chat'), false);
  settingsPlace.query = 'font';
  assert.equal(settingsPlace.arrive('chat'), true);
  assert.equal(settingsPlace.query, 'font');
  assert.equal(settingsPlace.arrive('accounts'), false);
  assert.equal(settingsPlace.query, '');
});

test('V107-Mo removed accounts are forgotten, other pages untouched', () => {
  settingsPlace.reset();
  settingsPlace.setOpen('accounts', 'profile:1', true);
  settingsPlace.setOpen('accounts', 'profile:2', true);
  settingsPlace.setOpen('filters', 'profile:1', true);
  settingsPlace.prune('accounts', 'profile:', ['2']);
  assert.equal(settingsPlace.open('accounts', 'profile:1'), false);
  assert.equal(settingsPlace.open('accounts', 'profile:2'), true);
  assert.equal(settingsPlace.open('filters', 'profile:1'), true);
});

test('V107-Mo the same key on two pages stays apart, and closing is kept', () => {
  settingsPlace.reset();
  settingsPlace.setOpen('a', 'x', true);
  assert.equal(settingsPlace.open('b', 'x'), false);
  settingsPlace.setOpen('a', 'x', false);
  assert.equal(settingsPlace.open('a', 'x', true), false);
});

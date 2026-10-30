import assert from 'node:assert/strict';
import { test } from 'node:test';

import { soundTip, tabSoundMode } from './tabSound.ts';

test('V106-25 a muted tab shows muted, playing or not', () => {
  assert.equal(tabSoundMode(true, true), 'muted');
  assert.equal(tabSoundMode(false, true), 'muted');
});

test('V106-25 an unmuted tab is audible only while something would be heard', () => {
  assert.equal(tabSoundMode(true, false), 'audible');
  assert.equal(tabSoundMode(false, false), 'silent');
});

test('V102-15 a held tab with every player muted shows crossed', () => {
  assert.equal(tabSoundMode(false, false, true), 'muted');
  assert.equal(tabSoundMode(false, false, false), 'silent');
  assert.equal(tabSoundMode(true, false, true), 'audible');
});

test('V106-25 no badge while no stream in the tab is on air, muted or not', () => {
  assert.equal(tabSoundMode(false, true, false, false), 'silent');
  assert.equal(tabSoundMode(true, false, true, false), 'silent');
  assert.equal(tabSoundMode(false, true, false, true), 'muted');
});

test('U2 the tip follows the badge', () => {
  assert.equal(soundTip('silent', false), '');
  assert.equal(soundTip('audible', false), ' · playing');
  assert.equal(soundTip('muted', false), ' · playing muted');
  assert.equal(soundTip('audible', true), ' · muted');
});
